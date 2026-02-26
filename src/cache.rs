use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, types::Value, Connection, Row};

use crate::paths;
use crate::types::Record;

const DB_FILENAME: &str = "usage.db";

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open() -> anyhow::Result<Self> {
        let db_path = Self::db_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-10000;
             PRAGMA temp_store=MEMORY;
             PRAGMA mmap_size=268435456;",
        )?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    fn db_path() -> PathBuf {
        paths::cache_dir().join(DB_FILENAME)
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_entries (
                id INTEGER PRIMARY KEY,
                provider TEXT NOT NULL,
                source_file TEXT NOT NULL,
                source_mtime INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                model TEXT,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                cache_creation_tokens INTEGER NOT NULL,
                thinking_tokens INTEGER NOT NULL,
                cost_usd REAL,
                message_id TEXT,
                request_id TEXT,
                session_id TEXT,
                dedup_key TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_timestamp ON usage_entries(timestamp);
            CREATE INDEX IF NOT EXISTS idx_source_file ON usage_entries(source_file, source_mtime);
            CREATE INDEX IF NOT EXISTS idx_provider_timestamp ON usage_entries(provider, timestamp);
            CREATE TABLE IF NOT EXISTS cache_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        // Migration: add preserved column if missing
        let has_preserved: bool = self
            .conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('usage_entries') WHERE name='preserved'")?
            .query_row([], |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;
        if !has_preserved {
            self.conn.execute_batch(
                "ALTER TABLE usage_entries ADD COLUMN preserved INTEGER NOT NULL DEFAULT 0;",
            )?;
        }

        Ok(())
    }

    pub fn begin(&self) -> anyhow::Result<()> {
        self.conn.execute_batch("BEGIN")?;
        Ok(())
    }

    pub fn commit(&self) -> anyhow::Result<()> {
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    /// Get all cached (file, mtime) pairs in one query for bulk staleness checking.
    pub fn cached_file_mtimes(&self) -> std::collections::HashMap<String, i64> {
        let mut map = std::collections::HashMap::new();
        let Ok(mut stmt) = self
            .conn
            .prepare("SELECT DISTINCT source_file, source_mtime FROM usage_entries")
        else {
            return map;
        };
        let Ok(rows) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }) else {
            return map;
        };
        for row in rows.flatten() {
            map.insert(row.0, row.1);
        }
        map
    }

    const ENTRY_COLUMNS: &str = "provider, timestamp, model, input_tokens, output_tokens, \
        cache_read_tokens, cache_creation_tokens, thinking_tokens, \
        cost_usd, message_id, request_id, session_id";

    /// Load ALL cached entries in one query.
    pub fn load_all_entries(&self) -> anyhow::Result<Vec<Record>> {
        let sql = format!(
            "SELECT {} FROM usage_entries ORDER BY timestamp",
            Self::ENTRY_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let entries = stmt
            .query_map([], Self::row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// Load cached entries with SQL-level filtering by date range and provider.
    pub fn load_entries_filtered(
        &self,
        since: Option<NaiveDate>,
        until: Option<NaiveDate>,
        providers: &[String],
    ) -> anyhow::Result<Vec<Record>> {
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Value> = Vec::new();

        if let Some(s) = since {
            conditions.push("timestamp >= ?".to_string());
            param_values.push(Value::Text(s.to_string()));
        }

        if let Some(u) = until {
            if let Some(next) = u.succ_opt() {
                conditions.push("timestamp < ?".to_string());
                param_values.push(Value::Text(next.to_string()));
            }
        }

        if !providers.is_empty() {
            let placeholders: Vec<&str> = providers.iter().map(|_| "?").collect();
            conditions.push(format!("provider IN ({})", placeholders.join(",")));
            for p in providers {
                param_values.push(Value::Text(p.clone()));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };
        let sql = format!(
            "SELECT {} FROM usage_entries{} ORDER BY timestamp",
            Self::ENTRY_COLUMNS,
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let entries = stmt
            .query_map(rusqlite::params_from_iter(param_values), Self::row_to_entry)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// Remove stale entries for a file and store new ones.
    pub fn store_file_entries(
        &self,
        path: &Path,
        mtime_secs: i64,
        entries: &[Record],
    ) -> anyhow::Result<()> {
        let path_str = path.display().to_string();

        self.conn.execute(
            "DELETE FROM usage_entries WHERE source_file = ?1",
            params![path_str],
        )?;

        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO usage_entries (
                provider, source_file, source_mtime, timestamp, model,
                input_tokens, output_tokens, cache_read_tokens,
                cache_creation_tokens, thinking_tokens, cost_usd,
                message_id, request_id, session_id, dedup_key
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )?;

        for entry in entries {
            stmt.execute(params![
                entry.provider,
                path_str,
                mtime_secs,
                entry.timestamp.to_rfc3339(),
                entry.model,
                entry.input_tokens,
                entry.output_tokens,
                entry.cache_read_tokens,
                entry.cache_creation_tokens,
                entry.thinking_tokens,
                entry.cost_usd,
                entry.message_id,
                entry.request_id,
                entry.session_id,
                entry.dedup_key(),
            ])?;
        }

        Ok(())
    }

    /// Check whether file discovery should be skipped because
    /// the cache was populated recently (within `max_age_secs`).
    #[must_use]
    pub fn should_rediscover(&self, max_age_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let last: Option<u64> = self
            .conn
            .query_row(
                "SELECT value FROM cache_meta WHERE key = 'last_discovery_at'",
                [],
                |row| {
                    let v: String = row.get(0)?;
                    Ok(v.parse::<u64>().unwrap_or(0))
                },
            )
            .ok();

        match last {
            Some(ts) => now.saturating_sub(ts) > max_age_secs,
            None => true,
        }
    }

    /// Record the current time as the last discovery timestamp.
    pub fn set_last_discovery(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('last_discovery_at', ?1)",
            params![now.to_string()],
        );
    }

    /// Mark entries as preserved when their source files no longer exist on disk.
    /// `discovered_files` is the set of currently-existing source file paths.
    pub fn mark_preserved(&self, discovered_files: &std::collections::HashSet<String>) {
        if discovered_files.is_empty() {
            return;
        }

        // Get all distinct source files in the cache
        let Ok(mut stmt) = self
            .conn
            .prepare("SELECT DISTINCT source_file FROM usage_entries WHERE preserved = 0")
        else {
            return;
        };
        let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
            return;
        };

        let cached_files: Vec<String> = rows.flatten().collect();
        for file in &cached_files {
            if !discovered_files.contains(file) {
                let _ = self.conn.execute(
                    "UPDATE usage_entries SET preserved = 1 WHERE source_file = ?1 AND preserved = 0",
                    params![file],
                );
            }
        }
    }

    /// Delete preserved entries with timestamps before the given date.
    pub fn prune_before(&self, before: NaiveDate) -> anyhow::Result<usize> {
        let before_str = before.to_string();
        let deleted = self.conn.execute(
            "DELETE FROM usage_entries WHERE preserved = 1 AND timestamp < ?1",
            params![before_str],
        )?;
        Ok(deleted)
    }

    /// Single source of truth for mapping a SQLite row to a Record.
    /// Column order must match ENTRY_COLUMNS.
    fn row_to_entry(row: &Row) -> rusqlite::Result<Record> {
        let ts_str: String = row.get(1)?;
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.to_utc())
            .unwrap_or_else(|_| Utc::now());

        Ok(Record {
            timestamp,
            provider: row.get(0)?,
            model: row.get(2)?,
            input_tokens: row.get::<_, i64>(3)? as u64,
            output_tokens: row.get::<_, i64>(4)? as u64,
            cache_read_tokens: row.get::<_, i64>(5)? as u64,
            cache_creation_tokens: row.get::<_, i64>(6)? as u64,
            thinking_tokens: row.get::<_, i64>(7)? as u64,
            cost_usd: row.get(8)?,
            message_id: row.get(9)?,
            request_id: row.get(10)?,
            session_id: row.get(11)?,
        })
    }
}

/// Get file modification time as seconds since epoch.
pub fn file_mtime_secs(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Create an in-memory cache for testing.
    fn test_cache() -> Cache {
        let conn = Connection::open_in_memory().unwrap();
        let cache = Cache { conn };
        cache.init_schema().unwrap();
        cache
    }

    fn make_record(provider: &str, timestamp: &str, session_id: Option<&str>) -> Record {
        Record {
            timestamp: DateTime::parse_from_rfc3339(timestamp)
                .unwrap()
                .to_utc(),
            provider: provider.to_string(),
            model: Some("test-model".to_string()),
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            thinking_tokens: 0,
            cost_usd: Some(0.01),
            message_id: None,
            request_id: None,
            session_id: session_id.map(String::from),
        }
    }

    #[test]
    fn test_store_and_load() {
        let cache = test_cache();
        let entries = vec![
            make_record("claude-code", "2026-02-20T10:00:00Z", Some("sess-1")),
            make_record("claude-code", "2026-02-21T10:00:00Z", Some("sess-1")),
        ];

        cache
            .store_file_entries(Path::new("/tmp/test.jsonl"), 1000, &entries)
            .unwrap();

        let loaded = cache.load_all_entries().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].provider, "claude-code");
        assert_eq!(loaded[0].session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_mark_preserved() {
        let cache = test_cache();

        // Store entries from two different files
        let entries_a = vec![make_record("claude-code", "2026-02-20T10:00:00Z", None)];
        let entries_b = vec![make_record("codex", "2026-02-21T10:00:00Z", None)];

        cache
            .store_file_entries(Path::new("/data/file_a.jsonl"), 1000, &entries_a)
            .unwrap();
        cache
            .store_file_entries(Path::new("/data/file_b.jsonl"), 2000, &entries_b)
            .unwrap();

        // Only file_a still exists on disk
        let discovered: HashSet<String> =
            ["/data/file_a.jsonl".to_string()].into_iter().collect();

        cache.mark_preserved(&discovered);

        // file_b's entries should be preserved=1
        let preserved_count: i64 = cache
            .conn
            .query_row(
                "SELECT COUNT(*) FROM usage_entries WHERE preserved = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved_count, 1);

        // file_a's entries should still be preserved=0
        let active_count: i64 = cache
            .conn
            .query_row(
                "SELECT COUNT(*) FROM usage_entries WHERE preserved = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 1);

        // All entries still load (preserved entries are regular rows)
        let all = cache.load_all_entries().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_mark_preserved_empty_discovered_is_noop() {
        let cache = test_cache();
        let entries = vec![make_record("claude-code", "2026-02-20T10:00:00Z", None)];
        cache
            .store_file_entries(Path::new("/data/file.jsonl"), 1000, &entries)
            .unwrap();

        // Empty discovered set should not mark anything
        cache.mark_preserved(&HashSet::new());

        let preserved_count: i64 = cache
            .conn
            .query_row(
                "SELECT COUNT(*) FROM usage_entries WHERE preserved = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved_count, 0);
    }

    #[test]
    fn test_prune_before() {
        let cache = test_cache();

        let entries = vec![
            make_record("claude-code", "2025-06-15T10:00:00Z", None),
            make_record("claude-code", "2026-02-20T10:00:00Z", None),
        ];
        cache
            .store_file_entries(Path::new("/data/old.jsonl"), 1000, &entries)
            .unwrap();

        // Mark all as preserved (simulating file deletion)
        // Empty discovered set would be a no-op due to guard, so mark manually
        cache
            .conn
            .execute(
                "UPDATE usage_entries SET preserved = 1",
                [],
            )
            .unwrap();

        // Prune entries before 2026-01-01
        let deleted = cache
            .prune_before(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
            .unwrap();
        assert_eq!(deleted, 1);

        // Only the 2026 entry should remain
        let remaining = cache.load_all_entries().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(
            remaining[0].timestamp.date_naive().to_string(),
            "2026-02-20"
        );
    }

    #[test]
    fn test_prune_ignores_non_preserved() {
        let cache = test_cache();

        let entries = vec![make_record("claude-code", "2025-06-15T10:00:00Z", None)];
        cache
            .store_file_entries(Path::new("/data/active.jsonl"), 1000, &entries)
            .unwrap();

        // preserved=0 (default), so prune should not delete it
        let deleted = cache
            .prune_before(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
            .unwrap();
        assert_eq!(deleted, 0);

        let remaining = cache.load_all_entries().unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn test_preserved_column_migration_idempotent() {
        // Calling init_schema twice should not error
        let cache = test_cache();
        cache.init_schema().unwrap();
        cache.init_schema().unwrap();
    }
}

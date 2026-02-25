use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::paths;
use crate::timestamp;
use crate::types::Record;

pub struct OpenCodeSource {
    db_path: PathBuf,
}

impl OpenCodeSource {
    pub fn new() -> Self {
        Self {
            db_path: paths::home_dir().join(".opencode/opencode.db"),
        }
    }
}

impl super::Source for OpenCodeSource {
    fn name(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "OpenCode"
    }

    fn data_dir(&self) -> PathBuf {
        self.db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    }

    fn discover_files(&self) -> Vec<PathBuf> {
        if self.db_path.exists() {
            vec![self.db_path.clone()]
        } else {
            Vec::new()
        }
    }

    fn parse_file(&self, path: &Path) -> Result<Vec<Record>> {
        let conn = match rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => {
                // Wait up to 5s if the DB is locked by a running OpenCode process
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(e) => {
                eprintln!("[tokemon] Warning: failed to open OpenCode DB: {}", e);
                return Ok(Vec::new());
            }
        };

        // Join sessions (which have token counts) with assistant messages (which have model names).
        // One record per session, using the model from the first assistant message.
        let mut stmt = match conn.prepare(
            "SELECT s.id, s.prompt_tokens, s.completion_tokens, s.cost, s.created_at,
                    (SELECT m.model FROM messages m
                     WHERE m.session_id = s.id AND m.role = 'assistant' AND m.model IS NOT NULL
                     LIMIT 1) as model
             FROM sessions s
             WHERE s.prompt_tokens > 0 OR s.completion_tokens > 0
             ORDER BY s.created_at",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[tokemon] Warning: failed to query OpenCode DB: {}", e);
                return Ok(Vec::new());
            }
        };

        let entries = stmt
            .query_map([], |row| {
                let session_id: String = row.get(0)?;
                let input_tokens: i64 = row.get(1)?;
                let output_tokens: i64 = row.get(2)?;
                let cost: f64 = row.get(3)?;
                let created_at: i64 = row.get(4)?;
                let model: Option<String> = row.get(5)?;
                Ok((
                    session_id,
                    input_tokens,
                    output_tokens,
                    cost,
                    created_at,
                    model,
                ))
            })
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                let (session_id, input_tokens, output_tokens, cost, created_at, model) =
                    row.ok()?;
                let ts = timestamp::parse_timestamp_numeric(created_at)?;
                Some(Record {
                    timestamp: ts,
                    provider: "opencode".to_string(),
                    model,
                    input_tokens: input_tokens as u64,
                    output_tokens: output_tokens as u64,
                    cache_read_tokens: 0,
                    cache_creation_tokens: 0,
                    thinking_tokens: 0,
                    cost_usd: if cost > 0.0 { Some(cost) } else { None },
                    message_id: None,
                    request_id: None,
                    session_id: Some(session_id),
                })
            })
            .collect();

        Ok(entries)
    }
}

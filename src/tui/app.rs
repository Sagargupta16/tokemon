use chrono::{Datelike, NaiveDate, Timelike, Utc};
use crossterm::event::{KeyCode, KeyEvent};

use crate::config::Config;
use crate::render::format_tokens_short;
use crate::types::{DailySummary, ModelUsage, Record};
use crate::{cost, rollup};

use super::diff::{self, RowChange};
use super::event::Event;

// ── View scope ────────────────────────────────────────────────────────────

/// Which time window the detail table shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Today,
    Week,
    Month,
}

impl Scope {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Week => "This Week",
            Self::Month => "This Month",
        }
    }

    /// Return the start date for this scope.
    #[must_use]
    pub fn since(self) -> NaiveDate {
        let today = Utc::now().date_naive();
        match self {
            Self::Today => today,
            Self::Week => {
                today - chrono::Duration::days(i64::from(today.weekday().num_days_from_monday()))
            }
            Self::Month => NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today),
        }
    }
}

// ── Summary card data ─────────────────────────────────────────────────────

/// Data for one summary card (Today / This Week / This Month).
#[derive(Debug, Clone)]
pub struct CardData {
    pub label: &'static str,
    pub cost: f64,
    pub tokens: u64,
    pub sparkline: Vec<u64>,
}

impl CardData {
    #[must_use]
    pub fn cost_str(&self) -> String {
        format_cost_compact(self.cost)
    }

    #[must_use]
    pub fn tokens_str(&self) -> String {
        format!("{} tokens", format_tokens_short(self.tokens))
    }
}

// ── App state ─────────────────────────────────────────────────────────────

pub struct App {
    /// Currently selected detail scope.
    pub scope: Scope,
    /// Whether to show per-model breakdown or compact rows.
    pub breakdown: bool,
    /// Whether history mode is toggled on.
    pub show_history: bool,
    /// Summary cards (always three: today, week, month).
    pub cards: [CardData; 3],
    /// Detail table rows for the selected scope.
    pub detail_models: Vec<ModelUsage>,
    /// Detail totals.
    pub detail_total_cost: f64,
    pub detail_total_tokens: u64,
    /// Historical summaries (populated when show_history is true).
    pub history_summaries: Vec<DailySummary>,
    /// Scroll offset for the detail table.
    pub scroll_offset: u16,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Whether the help overlay is shown.
    pub show_help: bool,
    /// Recent row changes detected by the diff engine (for animations).
    pub recent_changes: Vec<RowChange>,
    /// Config reference.
    config: Config,
    /// Cached raw records for the current data load.
    cached_records: Vec<Record>,
    /// Previous model snapshot for diffing.
    prev_models: Vec<ModelUsage>,
}

impl App {
    /// Create a new app and perform the initial data load.
    pub fn new(config: &Config, initial_scope: Scope) -> Self {
        let mut app = Self {
            scope: initial_scope,
            breakdown: true,
            show_history: false,
            cards: [
                CardData {
                    label: "Today",
                    cost: 0.0,
                    tokens: 0,
                    sparkline: Vec::new(),
                },
                CardData {
                    label: "This Week",
                    cost: 0.0,
                    tokens: 0,
                    sparkline: Vec::new(),
                },
                CardData {
                    label: "This Month",
                    cost: 0.0,
                    tokens: 0,
                    sparkline: Vec::new(),
                },
            ],
            detail_models: Vec::new(),
            detail_total_cost: 0.0,
            detail_total_tokens: 0,
            history_summaries: Vec::new(),
            scroll_offset: 0,
            should_quit: false,
            show_help: false,
            recent_changes: Vec::new(),
            config: config.clone(),
            cached_records: Vec::new(),
            prev_models: Vec::new(),
        };
        app.refresh_data();
        app
    }

    /// Handle an incoming event. Returns `true` if the UI needs a redraw.
    pub fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Tick | Event::DataChanged => {
                self.refresh_data();
                true
            }
            Event::Resize(_, _) => true,
            Event::Render => true,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // If help is shown, any key dismisses it
        if self.show_help {
            self.show_help = false;
            return true;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                false
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                true
            }
            KeyCode::Char('t') => {
                self.scope = Scope::Today;
                self.scroll_offset = 0;
                self.recompute_detail();
                true
            }
            KeyCode::Char('w') => {
                self.scope = Scope::Week;
                self.scroll_offset = 0;
                self.recompute_detail();
                true
            }
            KeyCode::Char('m') => {
                self.scope = Scope::Month;
                self.scroll_offset = 0;
                self.recompute_detail();
                true
            }
            KeyCode::Char('b') => {
                self.breakdown = !self.breakdown;
                true
            }
            KeyCode::Char('h') => {
                self.show_history = !self.show_history;
                self.recompute_detail();
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    /// Reload all data from the cache and recompute everything.
    pub fn refresh_data(&mut self) {
        // Load records from cache. We do a full load (no date filter)
        // so we can compute all three card summaries.
        let records = load_records_from_cache(&self.config);
        self.cached_records = records;
        self.recompute_cards();

        // Snapshot current models before recomputing for diff
        let prev = std::mem::take(&mut self.prev_models);
        self.recompute_detail();

        // Compute diff against previous state
        self.recent_changes = diff::diff(&prev, &self.detail_models);

        // Save current models for next diff
        self.prev_models = self.detail_models.clone();
    }

    fn recompute_cards(&mut self) {
        let today = Utc::now().date_naive();

        // Today card
        let today_records: Vec<&Record> = self
            .cached_records
            .iter()
            .filter(|r| r.timestamp.date_naive() == today)
            .collect();
        self.cards[0].cost = sum_cost(&today_records);
        self.cards[0].tokens = sum_tokens(&today_records);

        // This week card
        let week_start = Scope::Week.since();
        let week_records: Vec<&Record> = self
            .cached_records
            .iter()
            .filter(|r| r.timestamp.date_naive() >= week_start)
            .collect();
        self.cards[1].cost = sum_cost(&week_records);
        self.cards[1].tokens = sum_tokens(&week_records);

        // Build sparkline: daily totals for the last 7 days
        self.cards[1].sparkline = build_daily_sparkline(&self.cached_records, 7);

        // This month card
        let month_start = Scope::Month.since();
        let month_records: Vec<&Record> = self
            .cached_records
            .iter()
            .filter(|r| r.timestamp.date_naive() >= month_start)
            .collect();
        self.cards[2].cost = sum_cost(&month_records);
        self.cards[2].tokens = sum_tokens(&month_records);

        // Build sparkline: daily totals for the last 30 days
        self.cards[2].sparkline = build_daily_sparkline(&self.cached_records, 30);

        // Today sparkline: hourly totals for today
        self.cards[0].sparkline = build_hourly_sparkline(&self.cached_records);
    }

    fn recompute_detail(&mut self) {
        let since = self.scope.since();
        let filtered: Vec<Record> = self
            .cached_records
            .iter()
            .filter(|r| r.timestamp.date_naive() >= since)
            .cloned()
            .collect();

        // Aggregate into model-level breakdown for the selected scope
        let summaries = rollup::aggregate_daily(&filtered);

        // Flatten all model usages across all days in the scope
        let mut model_map: std::collections::HashMap<(String, String), ModelUsage> =
            std::collections::HashMap::new();

        for summary in &summaries {
            for mu in &summary.models {
                let key = (mu.model.clone(), mu.provider.clone());
                let entry = model_map.entry(key).or_insert_with(|| ModelUsage {
                    model: mu.model.clone(),
                    provider: mu.provider.clone(),
                    ..Default::default()
                });
                entry.input_tokens += mu.input_tokens;
                entry.output_tokens += mu.output_tokens;
                entry.cache_read_tokens += mu.cache_read_tokens;
                entry.cache_creation_tokens += mu.cache_creation_tokens;
                entry.thinking_tokens += mu.thinking_tokens;
                entry.cost_usd += mu.cost_usd;
                entry.request_count += mu.request_count;
            }
        }

        let mut models: Vec<ModelUsage> = model_map.into_values().collect();
        models.sort_unstable_by(|a, b| b.cost_usd.total_cmp(&a.cost_usd));

        self.detail_total_cost = models.iter().map(|m| m.cost_usd).sum();
        self.detail_total_tokens = models
            .iter()
            .map(|m| {
                m.input_tokens
                    + m.output_tokens
                    + m.cache_read_tokens
                    + m.cache_creation_tokens
                    + m.thinking_tokens
            })
            .sum();
        self.detail_models = models;

        // Historical summaries for the history view
        if self.show_history {
            self.history_summaries = match self.scope {
                Scope::Today => rollup::aggregate_daily(&filtered),
                Scope::Week => rollup::aggregate_daily(&filtered),
                Scope::Month => rollup::aggregate_weekly(&filtered),
            };
        } else {
            self.history_summaries.clear();
        }
    }
}

// ── Data loading ──────────────────────────────────────────────────────────

fn load_records_from_cache(config: &Config) -> Vec<Record> {
    use crate::cache::Cache;

    let cache = match Cache::open() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Load everything — the TUI filters in memory for card summaries.
    // We load the last 30 days to keep things bounded.
    let since = Scope::Month.since() - chrono::Duration::days(30);
    let mut entries = cache
        .load_entries_filtered(Some(since), None, &[])
        .unwrap_or_default();

    // Apply pricing
    if !config.no_cost {
        // Use offline pricing to avoid blocking the UI with HTTP requests
        if let Ok(engine) = cost::PricingEngine::load(true) {
            engine.apply_costs(&mut entries);
        }
    }

    entries.sort_by_key(|e| e.timestamp);
    entries
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn sum_cost(records: &[&Record]) -> f64 {
    records.iter().map(|r| r.cost_usd.unwrap_or(0.0)).sum()
}

fn sum_tokens(records: &[&Record]) -> u64 {
    records.iter().map(|r| r.total_tokens()).sum()
}

/// Build a sparkline of daily token totals for the last `days` days.
fn build_daily_sparkline(records: &[Record], days: usize) -> Vec<u64> {
    let today = Utc::now().date_naive();
    let mut data = vec![0u64; days];

    for record in records {
        let date = record.timestamp.date_naive();
        let offset = (today - date).num_days();
        if let Ok(idx) = usize::try_from(offset) {
            if idx < days {
                data[days - 1 - idx] += record.total_tokens();
            }
        }
    }

    data
}

/// Build a sparkline of hourly token totals for today (24 buckets).
fn build_hourly_sparkline(records: &[Record]) -> Vec<u64> {
    let today = Utc::now().date_naive();
    let mut data = vec![0u64; 24];

    for record in records {
        if record.timestamp.date_naive() == today {
            let hour = record.timestamp.hour() as usize;
            if hour < 24 {
                data[hour] += record.total_tokens();
            }
        }
    }

    data
}

fn format_cost_compact(cost: f64) -> String {
    if cost == 0.0 {
        "$0.00".to_string()
    } else if cost < 0.01 {
        format!("${cost:.4}")
    } else if cost >= 100.0 {
        format!("${cost:.0}")
    } else {
        format!("${cost:.2}")
    }
}

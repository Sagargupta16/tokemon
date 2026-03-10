mod app;
pub(crate) mod diff;
mod event;
mod terminal;
mod theme;
mod views;
mod watcher;
mod widgets;

use std::time::{Duration, Instant};

use tachyonfx::fx;
use tachyonfx::{EffectManager, Interpolation};

use crate::config::Config;
use app::{App, Scope};
use diff::ChangeKind;
use event::{Event, EventHandler};

/// Default data poll interval (seconds).
const DEFAULT_TICK_SECS: u64 = 2;

/// Target frame rate for rendering.
const RENDER_FPS: u64 = 30;

/// Run the TUI dashboard.
///
/// This is the entry point called from `main.rs` when the user runs
/// `tokemon top`. It sets up the terminal, event loop, and runs until
/// the user quits.
///
/// # Errors
///
/// Returns an error if terminal initialisation fails.
pub fn run(config: &Config, initial_view: &str, tick_interval: u64) -> anyhow::Result<()> {
    let scope = match initial_view {
        "week" | "w" => Scope::Week,
        "month" | "m" => Scope::Month,
        _ => Scope::Today,
    };

    let tick_secs = if tick_interval == 0 {
        DEFAULT_TICK_SECS
    } else {
        tick_interval
    };

    // Build a tokio runtime for the async event loop.
    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async {
        run_async(config, scope, tick_secs).await
    })
}

/// Effect keys for uniquely identified effects.
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum FxKey {
    /// Default variant (unused).
    #[default]
    None,
    /// View-switch dissolve effect on the table area.
    ViewSwitch,
    /// Row-level animation (keyed by model+provider).
    Row(String),
}

/// Create effects for data changes detected by the diff engine.
fn create_data_effects(app: &App, effects: &mut EffectManager<FxKey>) {
    for change in &app.recent_changes {
        let key = FxKey::Row(format!("{}:{}", change.key.model, change.key.provider));
        let effect = match change.kind {
            // New row: fade-in from blue background, 1.5s ease-out
            ChangeKind::New => fx::fade_from(
                theme::FLASH_NEW,
                theme::FLASH_NEW,
                (1500, Interpolation::QuadOut),
            ),
            // Updated row: flash green then fade back, 800ms ease-out
            ChangeKind::Updated => fx::fade_from(
                theme::FLASH_UPDATE,
                theme::FLASH_UPDATE,
                (800, Interpolation::QuadOut),
            ),
        };
        effects.add_unique_effect(key, effect);
    }
}

/// Create a view-switch dissolve effect.
fn create_view_switch_effect(effects: &mut EffectManager<FxKey>) {
    let effect = fx::sequence(&[
        fx::dissolve((100, Interpolation::QuadOut)),
        fx::coalesce((100, Interpolation::QuadIn)),
    ]);
    effects.add_unique_effect(FxKey::ViewSwitch, effect);
}

async fn run_async(config: &Config, scope: Scope, tick_secs: u64) -> anyhow::Result<()> {
    let mut terminal = terminal::init()?;
    let mut app = App::new(config, scope);
    let mut effects: EffectManager<FxKey> = EffectManager::default();
    let mut last_frame = Instant::now();

    let mut events = EventHandler::new(
        Duration::from_secs(tick_secs),
        Duration::from_millis(1000 / RENDER_FPS),
    );
    events.start();

    // Start the file watcher in the background.
    // It will send Event::DataChanged when source files are modified.
    let event_tx = events.sender();
    watcher::start(event_tx, config.no_cost);

    // Main loop
    loop {
        let Some(event) = events.next().await else {
            break;
        };

        let needs_draw = match &event {
            Event::Render => true,
            other => app.handle_event(other),
        };

        if app.should_quit {
            break;
        }

        // Create effects for any state changes
        if app.view_switched {
            create_view_switch_effect(&mut effects);
            app.view_switched = false;
        }
        if !app.recent_changes.is_empty() {
            create_data_effects(&app, &mut effects);
            app.recent_changes.clear();
        }

        // Draw if needed or if effects are running
        if needs_draw || effects.is_running() {
            let frame_duration = last_frame.elapsed();
            last_frame = Instant::now();

            terminal.draw(|frame| {
                let areas = views::dashboard::render(frame, &app);

                // Process tachyonfx effects on the rendered buffer
                effects.process_effects(
                    frame_duration.into(),
                    frame.buffer_mut(),
                    areas.table_area,
                );
            })?;
        }
    }

    terminal::restore()?;
    Ok(())
}

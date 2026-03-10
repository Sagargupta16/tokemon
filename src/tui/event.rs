use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

/// Application-level events.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Tick — time to poll for data updates.
    Tick,
    /// Render — time to redraw the UI.
    Render,
    /// The file watcher detected changes and updated the cache.
    DataChanged,
}

/// Drives the event loop, forwarding crossterm events and emitting periodic
/// tick / render events through an `mpsc` channel.
pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    tick_rate: Duration,
    render_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler.
    ///
    /// * `tick_rate` — how often to emit `Event::Tick` (data poll interval).
    /// * `render_rate` — how often to emit `Event::Render` (frame rate).
    #[must_use]
    pub fn new(tick_rate: Duration, render_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx,
            tick_rate,
            render_rate,
        }
    }

    /// Get a clone of the sender for external use (e.g. file watcher).
    #[must_use]
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    /// Start the background event loop. This spawns a tokio task that runs
    /// until the sender is dropped or the task is aborted.
    pub fn start(&self) {
        let tx = self.tx.clone();
        let tick_rate = self.tick_rate;
        let render_rate = self.render_rate;

        tokio::spawn(async move {
            let mut tick_interval = tokio::time::interval(tick_rate);
            let mut render_interval = tokio::time::interval(render_rate);

            // Don't let missed ticks pile up — skip them.
            tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    _ = render_interval.tick() => {
                        if tx.send(Event::Render).is_err() {
                            break;
                        }
                    }
                    // Poll crossterm events with a short timeout so we can
                    // interleave with tick/render.
                    _ = tokio::task::spawn_blocking(|| {
                        event::poll(Duration::from_millis(16))
                    }) => {
                        if let Ok(true) = event::poll(Duration::ZERO) {
                            if let Ok(evt) = event::read() {
                                let app_event = match evt {
                                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                                        Some(Event::Key(key))
                                    }
                                    CrosstermEvent::Resize(w, h) => Some(Event::Resize(w, h)),
                                    _ => None,
                                };
                                if let Some(e) = app_event {
                                    if tx.send(e).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Receive the next event. Returns `None` if the channel is closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

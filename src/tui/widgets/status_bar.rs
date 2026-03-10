use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::tui::theme;

/// Render the bottom status bar with keybinding hints.
pub fn render(frame: &mut Frame, area: Rect) {
    let bindings = vec![
        ("t", "today"),
        ("w", "week"),
        ("m", "month"),
        ("b", "breakdown"),
        ("h", "history"),
        ("j/k", "scroll"),
        ("q", "quit"),
        ("?", "help"),
    ];

    let mut spans: Vec<Span> = Vec::with_capacity(bindings.len() * 3);
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", theme::status_bar()));
        }
        spans.push(Span::styled(*key, theme::status_key()));
        spans.push(Span::styled(format!(" {}", desc), theme::status_bar()));
    }

    let line = Line::from(spans);

    // Fill the background with surface colour
    let bg = ratatui::widgets::Block::default().style(theme::status_bar());
    frame.render_widget(bg, area);
    frame.render_widget(line, area);
}

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

/// Render the bottom status bar with keybinding hints or filter input.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Fill the background with surface colour
    let bg = ratatui::widgets::Block::default().style(theme::status_bar());
    frame.render_widget(bg, area);

    if app.filter_active {
        // Show filter input
        let line = Line::from(vec![
            Span::styled("/", theme::status_key()),
            Span::styled(&app.filter_text, theme::text_bold()),
            Span::styled("█", theme::status_key()), // cursor
        ]);
        frame.render_widget(line, area);
        return;
    }

    let mut spans: Vec<Span> = Vec::new();

    // Show active filter if any
    if !app.applied_filter.is_empty() {
        spans.push(Span::styled("filter:", theme::status_key()));
        spans.push(Span::styled(
            format!("{} ", &app.applied_filter),
            ratatui::style::Style::default()
                .fg(theme::YELLOW)
                .bg(theme::SURFACE),
        ));
        spans.push(Span::styled(" │ ", theme::status_bar()));
    }

    let sort_label = format!("sort:{}", app.sort_order.label());
    let bindings: Vec<(&str, &str)> = vec![
        ("t", "today"),
        ("w", "week"),
        ("m", "month"),
        ("b", "breakdown"),
        ("h", "history"),
        ("/", "filter"),
        ("s", &sort_label),
        ("j/k", "scroll"),
        ("q", "quit"),
        ("?", "help"),
    ];

    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", theme::status_bar()));
        }
        spans.push(Span::styled(*key, theme::status_key()));
        spans.push(Span::styled(format!(" {desc}"), theme::status_bar()));
    }

    let line = Line::from(spans);
    frame.render_widget(line, area);
}

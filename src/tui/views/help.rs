use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::theme;

/// Render the help overlay as a centered popup.
pub fn render(frame: &mut Frame) {
    let area = frame.area();

    // Size the popup: 50 cols wide, 18 rows tall (or smaller if terminal is tiny)
    let popup_width = area.width.min(52);
    let popup_height = area.height.min(20);

    let [popup_area] = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .areas(
            Layout::vertical([Constraint::Length(popup_height)])
                .flex(Flex::Center)
                .areas::<1>(area)[0],
        );

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Help ", theme::header()))
        .borders(Borders::ALL)
        .border_style(theme::border().fg(theme::ACCENT))
        .style(theme::card());

    let inner = block.inner(popup_area);

    let bindings = vec![
        ("t", "Switch to Today view"),
        ("w", "Switch to This Week view"),
        ("m", "Switch to This Month view"),
        ("b", "Toggle Breakdown / Compact"),
        ("h", "Toggle historical rows"),
        ("j / ↓", "Scroll down"),
        ("k / ↑", "Scroll up"),
        ("?", "Toggle this help"),
        ("q / Esc", "Quit"),
        ("", ""),
        ("", "Data refreshes automatically when"),
        ("", "source files change on disk."),
    ];

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                Line::from(Span::styled(*desc, theme::text_dim()))
            } else {
                Line::from(vec![
                    Span::styled(format!("  {key:<10}"), theme::status_key()),
                    Span::styled(format!(" {desc}"), theme::text()),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines);

    frame.render_widget(block, popup_area);
    frame.render_widget(paragraph, inner);
}

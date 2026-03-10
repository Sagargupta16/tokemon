use chrono::Utc;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;

/// Render the top header bar.
///
/// ```text
/// ◆ tokemon                                        10 Mar 2026
/// ```
pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(16)]).areas(area);

    // Left: branding
    let brand = Line::from(vec![
        Span::styled("◆ ", theme::header()),
        Span::styled("tokemon", theme::text_bold()),
    ]);
    frame.render_widget(brand, left);

    // Right: current date
    let now = Utc::now();
    let date_str = now.format("%d %b %Y").to_string();
    let date_line = Line::from(Span::styled(date_str, theme::text_dim())).right_aligned();
    frame.render_widget(date_line, right);
}

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme;
use crate::tui::views::help;
use crate::tui::widgets::{header, status_bar, summary_cards, usage_table};

/// Areas returned by the dashboard render for effect targeting.
pub struct DashboardAreas {
    /// The area occupied by the usage table.
    pub table_area: Rect,
}

/// Render the complete dashboard view.
///
/// Layout:
/// ```text
/// ┌────────────── header (1 line) ──────────────┐
/// ├──────────── summary cards (7 lines) ────────┤
/// ├────────── usage table (flexible) ───────────┤
/// ├────────────── status bar (1 line) ──────────┤
/// └─────────────────────────────────────────────┘
/// ```
///
/// Returns the layout areas for effect targeting.
pub fn render(frame: &mut Frame, app: &App) -> DashboardAreas {
    let area = frame.area();

    // Fill the entire background
    let bg = Block::default().style(theme::text());
    frame.render_widget(bg, area);

    // Determine card height based on terminal height
    let card_height = if area.height >= 30 {
        7
    } else if area.height >= 20 {
        5
    } else {
        0 // Skip cards on very small terminals
    };

    let mut constraints = vec![
        Constraint::Length(1), // header
    ];

    if card_height > 0 {
        constraints.push(Constraint::Length(card_height)); // summary cards
    }

    constraints.push(Constraint::Min(5)); // usage table
    constraints.push(Constraint::Length(1)); // status bar

    let layout = Layout::vertical(constraints).split(area);

    let mut idx = 0;

    // Header
    header::render(frame, layout[idx], app);
    idx += 1;

    // Summary cards (if space)
    if card_height > 0 {
        summary_cards::render(frame, layout[idx], app);
        idx += 1;
    }

    // Usage table
    let table_area = layout[idx];
    usage_table::render(frame, table_area, app);
    idx += 1;

    // Status bar
    status_bar::render(frame, layout[idx], app);

    // Help overlay (rendered on top of everything)
    if app.show_help {
        help::render(frame);
    }

    DashboardAreas { table_area }
}

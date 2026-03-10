use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::app::{App, SettingField};
use crate::tui::theme;

/// Render the settings overlay as a centered popup.
#[allow(clippy::too_many_lines)]
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let state = &app.settings_state;

    // Size: wide enough for labels + values, tall enough for all fields + sections + footer
    let popup_width = area.width.min(60);
    let popup_height = area.height.min(32);

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Title with unsaved indicator
    let title = if state.unsaved {
        " Settings [modified] "
    } else {
        " Settings "
    };

    let block = Block::default()
        .title(Span::styled(title, theme::header()))
        .borders(Borders::ALL)
        .border_style(theme::border().fg(if state.unsaved {
            theme::YELLOW
        } else {
            theme::ACCENT
        }))
        .style(theme::card());

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Build the content lines, tracking which line index each field maps to.
    let mut lines: Vec<Line> = Vec::with_capacity(SettingField::COUNT + 10);
    let mut field_line_indices: Vec<usize> = Vec::with_capacity(SettingField::COUNT);

    for (idx, field) in SettingField::ALL.iter().enumerate() {
        // Section header
        if let Some(header) = field.section_header() {
            if idx > 0 {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!("  {header}"),
                theme::header(),
            )));
        }

        field_line_indices.push(lines.len()); // record the line index for this field

        let is_selected = idx == state.selected;
        let value_str = if state.editing && is_selected {
            // Show edit buffer with cursor
            format!("{}|", state.edit_buffer)
        } else {
            field.display_value(&state.draft)
        };

        // Format the value display based on field type
        let value_display = if field.is_bool() {
            let on = value_str == "Yes";
            if on { "[x]" } else { "[ ]" }.to_string()
        } else if field.is_enum() && is_selected {
            format!("<  {value_str}  >")
        } else {
            value_str
        };

        let label = field.label();

        // Calculate padding for right-aligned value
        let inner_width = inner.width as usize;
        let label_part = format!("  {label}");
        let padding = inner_width
            .saturating_sub(label_part.len())
            .saturating_sub(value_display.len())
            .saturating_sub(2); // right margin

        let line = if is_selected {
            let bg = theme::ACCENT_DIM;
            Line::from(vec![
                Span::styled(
                    label_part,
                    ratatui::style::Style::default()
                        .fg(theme::FG_BRIGHT)
                        .bg(bg)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled(" ".repeat(padding), ratatui::style::Style::default().bg(bg)),
                Span::styled(
                    value_display,
                    ratatui::style::Style::default()
                        .fg(if state.editing {
                            theme::YELLOW
                        } else {
                            theme::FG_BRIGHT
                        })
                        .bg(bg)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
                Span::styled("  ", ratatui::style::Style::default().bg(bg)),
            ])
        } else {
            Line::from(vec![
                Span::styled(label_part, theme::text_dim()),
                Span::styled(" ".repeat(padding), theme::card()),
                Span::styled(value_display, theme::text()),
                Span::styled("  ", theme::card()),
            ])
        };

        lines.push(line);
    }

    // Flash message
    if let Some((msg, _)) = &state.flash_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {msg}"),
            ratatui::style::Style::default()
                .fg(theme::GREEN)
                .bg(theme::SURFACE)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
    }

    // Footer
    lines.push(Line::from(""));
    let footer_line = if state.editing {
        Line::from(vec![
            Span::styled("  Enter", theme::status_key()),
            Span::styled(": Apply  ", theme::card_secondary()),
            Span::styled("Esc", theme::status_key()),
            Span::styled(": Cancel", theme::card_secondary()),
        ])
    } else {
        Line::from(vec![
            Span::styled("  W", theme::status_key()),
            Span::styled(": Save  ", theme::card_secondary()),
            Span::styled("Esc", theme::status_key()),
            Span::styled(": Close  ", theme::card_secondary()),
            Span::styled("\u{2191}\u{2193}", theme::status_key()),
            Span::styled(": Navigate  ", theme::card_secondary()),
            Span::styled("Enter", theme::status_key()),
            Span::styled(": Edit/Toggle", theme::card_secondary()),
        ])
    };
    lines.push(footer_line);

    // Determine scroll: if selected item is below visible area, scroll down
    let visible_height = inner.height as usize;
    let selected_line_idx = field_line_indices.get(state.selected).copied().unwrap_or(0);

    let scroll_offset = if selected_line_idx >= visible_height {
        selected_line_idx.saturating_sub(visible_height / 2)
    } else {
        0
    };

    #[allow(clippy::cast_possible_truncation)]
    let paragraph = Paragraph::new(lines).scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [popup_area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(
            Layout::vertical([Constraint::Length(height)])
                .flex(Flex::Center)
                .areas::<1>(area)[0],
        );
    popup_area
}

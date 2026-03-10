use ratatui::layout::{Constraint, Rect};
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::display;
use crate::render::format_tokens_short;
use crate::tui::app::App;
use crate::tui::theme;

/// Render the main usage detail table.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border())
        .title(Span::styled(
            format!(" {} ", app.scope.label()),
            theme::header(),
        ))
        .style(theme::text());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    // Determine which columns fit
    let cols = choose_columns(inner.width as usize);

    // Build header
    let header_cells: Vec<Cell> = cols
        .headers()
        .into_iter()
        .map(|h| Cell::from(Span::styled(h, theme::header())))
        .collect();
    let header = Row::new(header_cells).height(1);

    // Build data rows
    let mut rows: Vec<Row> = Vec::new();

    if app.show_history && !app.history_summaries.is_empty() {
        // History mode: show per-period summaries with sub-rows
        let today = chrono::Utc::now().date_naive();
        for summary in &app.history_summaries {
            let is_current = summary.date == today
                || (app.scope == crate::tui::app::Scope::Week
                    && summary.date >= crate::tui::app::Scope::Week.since())
                || (app.scope == crate::tui::app::Scope::Month
                    && summary.date >= crate::tui::app::Scope::Month.since());

            let style = if is_current {
                theme::text_bold()
            } else {
                theme::text_dim()
            };

            // Period summary row
            let total = summary.total_input
                + summary.total_output
                + summary.total_cache
                + summary.total_thinking;
            let period_cells = cols.build_row(
                &summary.label,
                "",
                "",
                summary.total_input,
                summary.total_output,
                total,
                summary.total_cost,
                style,
                is_current,
            );
            rows.push(Row::new(period_cells).height(1));

            // Model sub-rows (if breakdown mode)
            if app.breakdown {
                for mu in &summary.models {
                    let model_total = mu.input_tokens
                        + mu.output_tokens
                        + mu.cache_read_tokens
                        + mu.cache_creation_tokens
                        + mu.thinking_tokens;
                    let sub_cells = cols.build_row(
                        "",
                        &format!("  {}", display::display_model(&mu.model)),
                        &display::infer_api_provider(&mu.model),
                        mu.input_tokens,
                        mu.output_tokens,
                        model_total,
                        mu.cost_usd,
                        style,
                        is_current,
                    );
                    rows.push(Row::new(sub_cells).height(1));
                }
            }
        }
    } else {
        // Normal mode: flat model list for the scope
        for mu in &app.detail_models {
            let total = mu.input_tokens
                + mu.output_tokens
                + mu.cache_read_tokens
                + mu.cache_creation_tokens
                + mu.thinking_tokens;
            let cells = cols.build_row(
                &display::display_model(&mu.model),
                &display::infer_api_provider(&mu.model),
                &display::display_client(&mu.provider),
                mu.input_tokens,
                mu.output_tokens,
                total,
                mu.cost_usd,
                theme::text(),
                true,
            );
            rows.push(Row::new(cells).height(1));
        }
    }

    // Total row
    let total_cells = cols.build_total_row(app.detail_total_tokens, app.detail_total_cost);
    rows.push(Row::new(total_cells).height(1));

    let table = Table::new(rows, cols.widths())
        .header(header)
        .row_highlight_style(theme::text().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, inner);
}

// ── Column management ─────────────────────────────────────────────────────

/// Which columns to display, based on available width.
#[derive(Debug, Clone, Copy)]
struct ColumnSet {
    show_api: bool,
    show_client: bool,
    show_input: bool,
    show_output: bool,
}

impl ColumnSet {
    fn headers(&self) -> Vec<String> {
        let mut h = vec!["Model".to_string()];
        if self.show_api {
            h.push("API".to_string());
        }
        if self.show_client {
            h.push("Client".to_string());
        }
        if self.show_input {
            h.push("Input".to_string());
        }
        if self.show_output {
            h.push("Output".to_string());
        }
        h.push("Total".to_string());
        h.push("Cost".to_string());
        h
    }

    fn widths(&self) -> Vec<Constraint> {
        let mut w: Vec<Constraint> = vec![Constraint::Min(12)]; // Model
        if self.show_api {
            w.push(Constraint::Length(12));
        }
        if self.show_client {
            w.push(Constraint::Length(14));
        }
        if self.show_input {
            w.push(Constraint::Length(8));
        }
        if self.show_output {
            w.push(Constraint::Length(8));
        }
        w.push(Constraint::Length(8)); // Total
        w.push(Constraint::Length(10)); // Cost
        w
    }

    #[allow(clippy::too_many_arguments)]
    fn build_row(
        &self,
        col0: &str,
        col1: &str,
        col2: &str,
        input: u64,
        output: u64,
        total: u64,
        cost: f64,
        base_style: ratatui::style::Style,
        use_color: bool,
    ) -> Vec<Cell<'static>> {
        let mut cells: Vec<Cell> = Vec::new();

        // In history mode, col0=date/label, col1=model, col2=api.
        // In normal mode, col0=model, col1=api, col2=client.
        // The caller sets these appropriately.
        if !col1.is_empty() || col0.is_empty() {
            // History mode row: col0 is label
            cells.push(Cell::from(Span::styled(col0.to_string(), base_style)));
        } else {
            cells.push(Cell::from(Span::styled(col0.to_string(), base_style)));
        }

        if self.show_api {
            cells.push(Cell::from(Span::styled(col1.to_string(), base_style)));
        }
        if self.show_client {
            cells.push(Cell::from(Span::styled(col2.to_string(), base_style)));
        }
        if self.show_input {
            let s = format_tokens_short(input);
            let style = if use_color {
                theme::tokens_style(input)
            } else {
                base_style
            };
            cells.push(Cell::from(Span::styled(s, style)));
        }
        if self.show_output {
            let s = format_tokens_short(output);
            let style = if use_color {
                theme::tokens_style(output)
            } else {
                base_style
            };
            cells.push(Cell::from(Span::styled(s, style)));
        }

        let total_s = format_tokens_short(total);
        let total_style = if use_color {
            theme::tokens_style(total)
        } else {
            base_style
        };
        cells.push(Cell::from(Span::styled(total_s, total_style)));

        let cost_s = format_cost(cost);
        let cost_style = if use_color {
            theme::cost_style(cost)
        } else {
            base_style
        };
        cells.push(Cell::from(Span::styled(cost_s, cost_style)));

        cells
    }

    fn build_total_row(&self, total_tokens: u64, total_cost: f64) -> Vec<Cell<'static>> {
        let style = theme::total_row();
        let mut cells: Vec<Cell> = vec![Cell::from(Span::styled("TOTAL", style))];
        if self.show_api {
            cells.push(Cell::from(Span::styled("", style)));
        }
        if self.show_client {
            cells.push(Cell::from(Span::styled("", style)));
        }
        if self.show_input {
            cells.push(Cell::from(Span::styled("", style)));
        }
        if self.show_output {
            cells.push(Cell::from(Span::styled("", style)));
        }
        cells.push(Cell::from(Span::styled(
            format_tokens_short(total_tokens),
            style,
        )));
        cells.push(Cell::from(Span::styled(format_cost(total_cost), style)));
        cells
    }
}

/// Choose which columns to display based on terminal width.
fn choose_columns(width: usize) -> ColumnSet {
    if width >= 80 {
        ColumnSet {
            show_api: true,
            show_client: true,
            show_input: true,
            show_output: true,
        }
    } else if width >= 65 {
        ColumnSet {
            show_api: true,
            show_client: false,
            show_input: true,
            show_output: true,
        }
    } else if width >= 50 {
        ColumnSet {
            show_api: false,
            show_client: false,
            show_input: true,
            show_output: true,
        }
    } else {
        ColumnSet {
            show_api: false,
            show_client: false,
            show_input: false,
            show_output: false,
        }
    }
}

fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        "$0.00".to_string()
    } else if cost < 0.01 {
        format!("${cost:.4}")
    } else {
        format!("${cost:.2}")
    }
}

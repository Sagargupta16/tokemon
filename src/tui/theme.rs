use ratatui::style::{Color, Modifier, Style};

// ── Base palette ──────────────────────────────────────────────────────────

/// Deep background — the terminal canvas.
pub const BG: Color = Color::Rgb(15, 17, 22);

/// Slightly lighter surface for panels / cards.
pub const SURFACE: Color = Color::Rgb(22, 25, 33);

/// Borders, separators.
pub const BORDER: Color = Color::Rgb(48, 54, 68);

/// Subtle text (labels, inactive items).
pub const DIM: Color = Color::Rgb(88, 96, 112);

/// Normal text.
pub const FG: Color = Color::Rgb(200, 205, 215);

/// Bright / emphasized text.
pub const FG_BRIGHT: Color = Color::Rgb(235, 238, 245);

// ── Accent colours ────────────────────────────────────────────────────────

/// Primary accent — brand / active tab / highlights.
pub const ACCENT: Color = Color::Rgb(99, 140, 255);

/// Secondary accent — less prominent highlights.
pub const ACCENT_DIM: Color = Color::Rgb(65, 95, 180);

/// Success / positive values.
pub const GREEN: Color = Color::Rgb(80, 200, 120);

/// Warning / moderate values.
pub const YELLOW: Color = Color::Rgb(230, 190, 60);

/// Error / high values.
pub const RED: Color = Color::Rgb(235, 85, 85);

/// Cyan for headers and labels.
pub const CYAN: Color = Color::Rgb(85, 205, 220);

/// New-row flash colour.
pub const FLASH_NEW: Color = Color::Rgb(60, 90, 180);

/// Update-row flash colour.
pub const FLASH_UPDATE: Color = Color::Rgb(50, 160, 90);

// ── Composite styles ──────────────────────────────────────────────────────

/// Default text style.
#[must_use]
pub fn text() -> Style {
    Style::default().fg(FG).bg(BG)
}

/// Dimmed / secondary text.
#[must_use]
pub fn text_dim() -> Style {
    Style::default().fg(DIM).bg(BG)
}

/// Bold bright text.
#[must_use]
pub fn text_bold() -> Style {
    Style::default()
        .fg(FG_BRIGHT)
        .bg(BG)
        .add_modifier(Modifier::BOLD)
}

/// Header / column label style.
#[must_use]
pub fn header() -> Style {
    Style::default()
        .fg(CYAN)
        .bg(BG)
        .add_modifier(Modifier::BOLD)
}

/// Active tab indicator.
#[must_use]
pub fn tab_active() -> Style {
    Style::default()
        .fg(BG)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

/// Inactive tab.
#[must_use]
pub fn tab_inactive() -> Style {
    Style::default().fg(DIM).bg(SURFACE)
}

/// Border style.
#[must_use]
pub fn border() -> Style {
    Style::default().fg(BORDER).bg(BG)
}

/// Cost styling based on value.
#[must_use]
pub fn cost_style(cost: f64) -> Style {
    let fg = if cost == 0.0 {
        DIM
    } else if cost < 1.0 {
        GREEN
    } else if cost < 10.0 {
        YELLOW
    } else {
        RED
    };
    Style::default().fg(fg).bg(BG)
}

/// Token count styling — dim for zeros.
#[must_use]
pub fn tokens_style(n: u64) -> Style {
    if n == 0 {
        text_dim()
    } else {
        text()
    }
}

/// Surface panel style (for cards).
#[must_use]
pub fn card() -> Style {
    Style::default().fg(FG).bg(SURFACE)
}

/// Card title / label.
#[must_use]
pub fn card_label() -> Style {
    Style::default()
        .fg(DIM)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

/// Card value (large number).
#[must_use]
pub fn card_value() -> Style {
    Style::default()
        .fg(FG_BRIGHT)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

/// Card secondary value (tokens).
#[must_use]
pub fn card_secondary() -> Style {
    Style::default().fg(DIM).bg(SURFACE)
}

/// Status bar background.
#[must_use]
pub fn status_bar() -> Style {
    Style::default().fg(DIM).bg(SURFACE)
}

/// Status bar keybinding highlight.
#[must_use]
pub fn status_key() -> Style {
    Style::default()
        .fg(ACCENT)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

/// Total row (bold).
#[must_use]
pub fn total_row() -> Style {
    Style::default()
        .fg(FG_BRIGHT)
        .bg(BG)
        .add_modifier(Modifier::BOLD)
}

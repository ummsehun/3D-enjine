use ratatui::prelude::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct StartUiTheme {
    pub accent: Color,
    pub focus: Color,
    pub success: Color,
    pub border_active: Style,
    pub border_idle: Style,
    pub text_primary: Style,
    pub text_secondary: Style,
    pub text_dim: Style,
    pub list_selected: Style,
    pub list_selected_symbol: &'static str,
    pub badge_quick: Style,
    pub badge_advanced: Style,
}

pub(super) fn start_ui_theme() -> StartUiTheme {
    StartUiTheme {
        accent: Color::Cyan,
        focus: Color::Yellow,
        success: Color::Green,
        border_active: Style::default().fg(Color::Cyan),
        border_idle: Style::default().fg(Color::DarkGray),
        text_primary: Style::default().fg(Color::White),
        text_secondary: Style::default().fg(Color::Gray),
        text_dim: Style::default().fg(Color::DarkGray),
        list_selected: Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        list_selected_symbol: "▸ ",
        badge_quick: Style::default().fg(Color::Black).bg(Color::LightCyan),
        badge_advanced: Style::default().fg(Color::Black).bg(Color::LightYellow),
    }
}

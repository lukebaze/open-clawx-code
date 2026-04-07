use ratatui::style::Color;

/// Color scheme for the TUI
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub border: Color,
    pub border_focused: Color,
    pub status_bg: Color,
    #[allow(dead_code)]
    pub status_fg: Color,
    pub input_bg: Color,
    pub input_fg: Color,
    pub dim: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(24, 24, 32),
            fg: Color::Rgb(220, 220, 230),
            accent: Color::Rgb(130, 170, 255),
            border: Color::Rgb(60, 60, 80),
            border_focused: Color::Rgb(130, 170, 255),
            status_bg: Color::Rgb(35, 35, 50),
            status_fg: Color::Rgb(180, 180, 200),
            input_bg: Color::Rgb(30, 30, 42),
            input_fg: Color::Rgb(220, 220, 230),
            dim: Color::Rgb(100, 100, 120),
        }
    }
}

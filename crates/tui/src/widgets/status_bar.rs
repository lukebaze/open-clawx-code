use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::input::InputMode;
use crate::theme::Theme;

/// Top status bar showing app name, mode indicator, and pending key
pub struct StatusBar<'a> {
    theme: &'a Theme,
    mode: InputMode,
    pending_key: bool,
}

impl<'a> StatusBar<'a> {
    pub const fn new(theme: &'a Theme, mode: InputMode, pending_key: bool) -> Self {
        Self {
            theme,
            mode,
            pending_key,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::new().bg(self.theme.status_bg));

        let mode_style = match self.mode {
            InputMode::Normal => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(100, 220, 140)),
            InputMode::Insert | InputMode::SlashComplete => Style::new()
                .fg(self.theme.bg)
                .bg(self.theme.accent),
            InputMode::Help => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(255, 200, 100)),
        };

        let mut spans = vec![
            Span::styled(" OCX ", Style::new().fg(self.theme.accent).bg(self.theme.status_bg)),
            Span::styled(format!(" {} ", self.mode.label()), mode_style),
        ];

        if self.pending_key {
            spans.push(Span::styled(
                " g-",
                Style::new().fg(self.theme.dim).bg(self.theme.status_bg),
            ));
        }

        spans.push(Span::styled(
            " | ? for help",
            Style::new().fg(self.theme.dim).bg(self.theme.status_bg),
        ));

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

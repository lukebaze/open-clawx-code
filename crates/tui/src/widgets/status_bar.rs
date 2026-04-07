use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::theme::Theme;

/// Top status bar showing app name, mode, and session info
pub struct StatusBar<'a> {
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    pub const fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill background
        buf.set_style(area, Style::new().bg(self.theme.status_bg));

        let left = Span::styled(
            " OCX ",
            Style::new().fg(self.theme.accent).bg(self.theme.status_bg),
        );
        let mode = Span::styled(
            " ready ",
            Style::new().fg(self.theme.status_fg).bg(self.theme.status_bg),
        );
        let line = Line::from(vec![left, mode]);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

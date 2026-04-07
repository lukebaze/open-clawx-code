use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::theme::Theme;

/// Bottom input bar for user prompts
pub struct InputBar<'a> {
    theme: &'a Theme,
    focused: bool,
}

impl<'a> InputBar<'a> {
    pub const fn new(theme: &'a Theme, focused: bool) -> Self {
        Self { theme, focused }
    }
}

impl Widget for InputBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(self.theme.input_bg).fg(self.theme.input_fg))
            .title(" > ");

        let paragraph = Paragraph::new("Type a message...")
            .style(Style::new().fg(self.theme.dim))
            .block(block);

        paragraph.render(area, buf);
    }
}

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::theme::Theme;

/// Placeholder panel used for conversation and context areas before real content
#[allow(dead_code)]
pub struct PlaceholderPanel<'a> {
    title: &'a str,
    theme: &'a Theme,
    focused: bool,
}

#[allow(dead_code)]
impl<'a> PlaceholderPanel<'a> {
    pub const fn new(title: &'a str, theme: &'a Theme, focused: bool) -> Self {
        Self {
            title,
            theme,
            focused,
        }
    }
}

impl Widget for PlaceholderPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(self.theme.bg).fg(self.theme.fg))
            .title(format!(" {} ", self.title));

        let paragraph = Paragraph::new(format!("{} panel", self.title))
            .style(Style::new().fg(self.theme.dim))
            .block(block);

        paragraph.render(area, buf);
    }
}

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::theme::Theme;

/// Bottom input bar for user prompts with editable text buffer
pub struct InputBar<'a> {
    theme: &'a Theme,
    focused: bool,
    text: &'a str,
    cursor_pos: usize,
}

impl<'a> InputBar<'a> {
    pub const fn new(theme: &'a Theme, focused: bool, text: &'a str, cursor_pos: usize) -> Self {
        Self {
            theme,
            focused,
            text,
            cursor_pos,
        }
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

        let display_text = if self.text.is_empty() {
            "Type a message...".to_string()
        } else {
            self.text.to_string()
        };

        let text_style = if self.text.is_empty() {
            Style::new().fg(self.theme.dim)
        } else {
            Style::new().fg(self.theme.input_fg)
        };

        let paragraph = Paragraph::new(display_text).style(text_style).block(block);
        paragraph.render(area, buf);

        // Render cursor position if focused and has text
        if self.focused {
            let cursor_x = area.x + 1 + u16::try_from(self.cursor_pos).unwrap_or(u16::MAX);
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                buf.set_style(
                    Rect::new(cursor_x, cursor_y, 1, 1),
                    Style::new().bg(self.theme.fg).fg(self.theme.bg),
                );
            }
        }
    }
}

/// Editable text buffer for the input bar
pub struct InputBuffer {
    text: String,
    cursor: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous character boundary
            let prev = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.text.len() {
            let next = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.text.len(), |(i, _)| self.cursor + i);
            self.text.drain(self.cursor..next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.text.len(), |(i, _)| self.cursor + i);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Take the current text, clearing the buffer
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

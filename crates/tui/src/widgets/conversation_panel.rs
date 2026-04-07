use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::conversation::{ConversationView, Role};
use crate::theme::Theme;

/// Renders the conversation history with role-based styling
pub struct ConversationPanel<'a> {
    conversation: &'a ConversationView,
    theme: &'a Theme,
    focused: bool,
}

impl<'a> ConversationPanel<'a> {
    pub const fn new(
        conversation: &'a ConversationView,
        theme: &'a Theme,
        focused: bool,
    ) -> Self {
        Self {
            conversation,
            theme,
            focused,
        }
    }

    fn render_messages(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        for msg in self.conversation.messages() {
            // Role label
            let (label, label_style) = match msg.role {
                Role::User => (
                    "You",
                    Style::new().fg(self.theme.accent),
                ),
                Role::Assistant => (
                    "Assistant",
                    Style::new().fg(ratatui::style::Color::Rgb(100, 220, 140)),
                ),
                Role::System => (
                    "System",
                    Style::new().fg(if msg.is_error {
                        ratatui::style::Color::Rgb(255, 100, 100)
                    } else {
                        self.theme.dim
                    }),
                ),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{label}: "), label_style),
            ]));

            // Message content — split into lines
            let content_style = if msg.is_error {
                Style::new().fg(ratatui::style::Color::Rgb(255, 100, 100))
            } else {
                Style::new().fg(self.theme.fg)
            };

            for line in msg.content.lines() {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    content_style,
                )));
            }

            // Usage metadata
            if let Some(ref meta) = msg.meta {
                lines.push(Line::from(Span::styled(
                    format!("  [tokens: {}in / {}out]", meta.input_tokens, meta.output_tokens),
                    Style::new().fg(self.theme.dim),
                )));
            }

            // Blank separator line
            lines.push(Line::from(""));
        }

        // Loading indicator
        if self.conversation.is_streaming {
            lines.push(Line::from(Span::styled(
                "  thinking...",
                Style::new().fg(self.theme.dim),
            )));
        }

        lines
    }
}

impl Widget for ConversationPanel<'_> {
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
            .title(" Conversation ");

        let lines = self.render_messages();

        // Calculate scroll: show bottom of conversation, offset by scroll_offset
        let visible_height = area.height.saturating_sub(2) as usize; // minus borders
        let total_lines = lines.len();
        let scroll_offset = self.conversation.scroll_offset();

        let skip = if total_lines > visible_height + scroll_offset {
            total_lines - visible_height - scroll_offset
        } else {
            0
        };

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(skip).unwrap_or(u16::MAX), 0));

        paragraph.render(area, buf);
    }
}

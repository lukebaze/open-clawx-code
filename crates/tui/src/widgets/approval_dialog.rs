use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::theme::Theme;

/// Centered dialog asking the user to approve or deny a tool call.
pub struct ApprovalDialog<'a> {
    pub tool_name: &'a str,
    pub input_summary: &'a str,
    pub theme: &'a Theme,
}

impl Widget for ApprovalDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 56.min(area.width.saturating_sub(4));
        let height = 7.min(area.height.saturating_sub(2));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog = Rect::new(x, y, width, height);

        Clear.render(dialog, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(ratatui::style::Color::Rgb(255, 200, 100)))
            .title(" Tool Approval ");
        let inner = block.inner(dialog);
        block.render(dialog, buf);

        // Line 1: tool name
        let name_line = Line::from(vec![
            Span::styled("  Tool: ", Style::new().fg(self.theme.dim)),
            Span::styled(
                self.tool_name,
                Style::new()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        buf.set_line(inner.x, inner.y, &name_line, inner.width);

        // Line 2: input summary (truncated)
        let max_len = (inner.width as usize).saturating_sub(10);
        let summary = if self.input_summary.len() > max_len {
            format!("{}…", &self.input_summary[..max_len.saturating_sub(1)])
        } else {
            self.input_summary.to_string()
        };
        let input_line = Line::from(vec![
            Span::styled("  Input: ", Style::new().fg(self.theme.dim)),
            Span::styled(summary, Style::new().fg(self.theme.fg)),
        ]);
        if inner.height > 1 {
            buf.set_line(inner.x, inner.y + 1, &input_line, inner.width);
        }

        // Line 4: prompt
        let prompt_line = Line::from(vec![
            Span::styled("  Allow? ", Style::new().fg(self.theme.fg)),
            Span::styled(
                "[Y]es",
                Style::new()
                    .fg(ratatui::style::Color::Rgb(100, 220, 140))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" / ", Style::new().fg(self.theme.dim)),
            Span::styled(
                "[n]o",
                Style::new()
                    .fg(ratatui::style::Color::Rgb(255, 100, 100))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        if inner.height > 3 {
            buf.set_line(inner.x, inner.y + 3, &prompt_line, inner.width);
        }
    }
}

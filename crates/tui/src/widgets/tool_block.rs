use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::theme::Theme;

#[allow(dead_code)]
const MAX_DISPLAY_LINES: usize = 500;

/// Inline tool invocation block in the conversation panel
#[allow(dead_code)]
pub struct ToolBlockWidget<'a> {
    pub tool_name: &'a str,
    pub is_expanded: bool,
    pub is_running: bool,
    pub result: Option<&'a str>,
    pub theme: &'a Theme,
}

impl Widget for ToolBlockWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.is_running {
            let line = Line::from(vec![
                Span::styled("  [", Style::new().fg(self.theme.dim)),
                Span::styled(self.tool_name, Style::new().fg(self.theme.accent)),
                Span::styled("] ⏳ running...", Style::new().fg(self.theme.dim)),
            ]);
            buf.set_line(area.x, area.y, &line, area.width);
            return;
        }

        let indicator = if self.is_expanded { "▼" } else { "▶" };
        let summary = self
            .result
            .map_or("(no output)", |r| {
                let first_line = r.lines().next().unwrap_or("");
                if first_line.len() > 80 {
                    &first_line[..80]
                } else {
                    first_line
                }
            });

        if self.is_expanded {
            // Expanded: bordered block with content
            let content = self.result.unwrap_or("");
            let lines: Vec<Line<'_>> = content
                .lines()
                .take(MAX_DISPLAY_LINES)
                .map(|l| Line::from(Span::styled(l, Style::new().fg(self.theme.fg))))
                .collect();

            let total_lines = content.lines().count();
            let mut display_lines = lines;
            if total_lines > MAX_DISPLAY_LINES {
                display_lines.push(Line::from(Span::styled(
                    format!("  ... ({} more lines)", total_lines - MAX_DISPLAY_LINES),
                    Style::new().fg(self.theme.dim),
                )));
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(self.theme.border))
                .title(format!(" {indicator} {} ", self.tool_name));

            let paragraph = Paragraph::new(display_lines)
                .block(block)
                .wrap(Wrap { trim: false });

            paragraph.render(area, buf);
        } else {
            // Collapsed: single line
            let line = Line::from(vec![
                Span::styled("  [", Style::new().fg(self.theme.dim)),
                Span::styled(self.tool_name, Style::new().fg(self.theme.accent)),
                Span::styled(
                    format!("] {indicator} {summary}"),
                    Style::new().fg(self.theme.dim),
                ),
            ]);
            buf.set_line(area.x, area.y, &line, area.width);
        }
    }
}

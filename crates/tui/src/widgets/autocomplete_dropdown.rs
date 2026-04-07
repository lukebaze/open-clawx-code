use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::input::slash_commands::SlashCompleter;
use crate::theme::Theme;

/// Floating autocomplete dropdown below the input bar
pub struct AutocompleteDropdown<'a> {
    pub completer: &'a SlashCompleter,
    pub theme: &'a Theme,
}

impl Widget for AutocompleteDropdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items = self.completer.filtered_commands();
        if items.is_empty() {
            return;
        }

        // Position: overlay above the input bar area
        let height = (u16::try_from(items.len()).unwrap_or(u16::MAX).saturating_add(2)).min(area.height);
        let width = area.width.min(50);
        let x = area.x + 1;
        let y = area.y.saturating_sub(height);

        let dropdown_area = Rect::new(x, y, width, height);

        // Clear background
        Clear.render(dropdown_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.theme.border_focused))
            .style(Style::new().bg(self.theme.status_bg));

        let lines: Vec<Line<'_>> = items
            .iter()
            .map(|&(name, desc, selected)| {
                let style = if selected {
                    Style::new().fg(self.theme.bg).bg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.fg)
                };
                Line::from(vec![
                    Span::styled(format!(" {name:<20}"), style),
                    Span::styled(
                        format!(" {desc}"),
                        if selected {
                            style
                        } else {
                            Style::new().fg(self.theme.dim)
                        },
                    ),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(dropdown_area, buf);
    }
}

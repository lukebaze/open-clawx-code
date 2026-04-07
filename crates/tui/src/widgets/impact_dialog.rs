use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use ocx_gitnexus::{ImpactResult, RiskLevel};

use crate::theme::Theme;

/// Centered dialog showing `GitNexus` impact analysis for a pre-edit gate.
pub struct ImpactDialog<'a> {
    pub impact: &'a ImpactResult,
    pub theme: &'a Theme,
}

impl Widget for ImpactDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 62.min(area.width.saturating_sub(4));
        let callers_count = self.impact.callers.len().min(5);
        #[allow(clippy::cast_possible_truncation)]
        let content_height = (callers_count + 7) as u16;
        let height = content_height.min(area.height.saturating_sub(2)).max(8);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog = Rect::new(x, y, width, height);

        Clear.render(dialog, buf);

        let border_color = match self.impact.risk_level {
            RiskLevel::High => ratatui::style::Color::Rgb(255, 200, 100),
            RiskLevel::Critical => ratatui::style::Color::Rgb(255, 80, 80),
            _ => ratatui::style::Color::Rgb(130, 170, 255),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(" Impact Analysis — Pre-Edit Gate ");
        let inner = block.inner(dialog);
        block.render(dialog, buf);

        let mut row = 0_u16;

        // Risk badge
        let risk_style = match self.impact.risk_level {
            RiskLevel::Low => Style::new().fg(ratatui::style::Color::Rgb(100, 220, 140)),
            RiskLevel::Medium => Style::new().fg(ratatui::style::Color::Rgb(130, 170, 255)),
            RiskLevel::High => Style::new()
                .fg(ratatui::style::Color::Rgb(255, 200, 100))
                .add_modifier(Modifier::BOLD),
            RiskLevel::Critical => Style::new()
                .fg(ratatui::style::Color::Rgb(255, 80, 80))
                .add_modifier(Modifier::BOLD),
        };

        let risk_line = Line::from(vec![
            Span::styled("  Risk: ", Style::new().fg(self.theme.dim)),
            Span::styled(self.impact.risk_level.label(), risk_style),
            Span::styled(
                format!("  Symbol: {}", self.impact.symbol),
                Style::new().fg(self.theme.fg),
            ),
        ]);
        buf.set_line(inner.x, inner.y + row, &risk_line, inner.width);
        row += 1;

        // Affected files count
        let files_line = Line::from(vec![
            Span::styled("  Affected files: ", Style::new().fg(self.theme.dim)),
            Span::styled(
                format!("{}", self.impact.affected_files.len()),
                Style::new().fg(self.theme.fg),
            ),
        ]);
        buf.set_line(inner.x, inner.y + row, &files_line, inner.width);
        row += 1;

        // Callers (up to 5)
        if !self.impact.callers.is_empty() {
            row += 1;
            let callers_header = Line::from(Span::styled(
                "  Direct callers:",
                Style::new().fg(self.theme.dim),
            ));
            buf.set_line(inner.x, inner.y + row, &callers_header, inner.width);
            row += 1;

            for caller in self.impact.callers.iter().take(5) {
                if inner.y + row >= inner.y + inner.height {
                    break;
                }
                let caller_line = Line::from(Span::styled(
                    format!("    {} ({})", caller.name, caller.file),
                    Style::new().fg(self.theme.fg),
                ));
                buf.set_line(inner.x, inner.y + row, &caller_line, inner.width);
                row += 1;
            }
        }

        // Prompt
        if inner.y + row + 1 < inner.y + inner.height {
            row = inner.height.saturating_sub(1);
            let prompt_line = Line::from(vec![
                Span::styled("  Proceed with edit? ", Style::new().fg(self.theme.fg)),
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
            buf.set_line(inner.x, inner.y + row, &prompt_line, inner.width);
        }
    }
}

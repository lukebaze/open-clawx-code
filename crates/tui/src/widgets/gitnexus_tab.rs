use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use ocx_gitnexus::{ImpactResult, RiskLevel};

use crate::theme::Theme;

/// `GitNexus` tab in the right-side context panel.
/// Shows recent impact analyses and code intelligence results.
#[allow(dead_code)]
pub struct GitNexusTabWidget<'a> {
    pub tab: &'a GitNexusTab,
    pub theme: &'a Theme,
    pub focused: bool,
}

impl Widget for GitNexusTabWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let available = if self.tab.available {
            ""
        } else {
            " (not installed)"
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(format!(" GitNexus{available} "));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.tab.impacts.is_empty() {
            let msg = if self.tab.available {
                "No impact analyses yet. Build mode triggers these automatically."
            } else {
                "GitNexus not installed. Run `npx gitnexus analyze` to set up."
            };
            let p = Paragraph::new(msg).style(Style::new().fg(self.theme.dim));
            p.render(inner, buf);
            return;
        }

        // Render impact list
        for (i, impact) in self.tab.impacts.iter().rev().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let row = i as u16;
            if row >= inner.height {
                break;
            }

            let is_selected = i == self.tab.selected;
            let risk_color = match impact.risk_level {
                RiskLevel::Low => ratatui::style::Color::Rgb(100, 220, 140),
                RiskLevel::Medium => ratatui::style::Color::Rgb(130, 170, 255),
                RiskLevel::High => ratatui::style::Color::Rgb(255, 200, 100),
                RiskLevel::Critical => ratatui::style::Color::Rgb(255, 80, 80),
            };

            let base_style = if is_selected {
                Style::new()
                    .fg(self.theme.bg)
                    .bg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(self.theme.fg)
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:>4} ", impact.risk_level.label()),
                    if is_selected {
                        base_style
                    } else {
                        Style::new().fg(risk_color)
                    },
                ),
                Span::styled(&impact.symbol, base_style),
                Span::styled(
                    format!(
                        "  {}f {}c",
                        impact.affected_files.len(),
                        impact.callers.len()
                    ),
                    if is_selected {
                        base_style
                    } else {
                        Style::new().fg(self.theme.dim)
                    },
                ),
            ]);
            buf.set_line(inner.x, inner.y + row, &line, inner.width);
        }
    }
}

/// State for the `GitNexus` context panel tab.
#[allow(dead_code)]
pub struct GitNexusTab {
    pub impacts: Vec<ImpactResult>,
    pub selected: usize,
    pub available: bool,
}

#[allow(dead_code)]
impl GitNexusTab {
    #[must_use]
    pub fn new(available: bool) -> Self {
        Self {
            impacts: Vec::new(),
            selected: 0,
            available,
        }
    }

    /// Add a new impact result (from orchestrator event).
    pub fn push_impact(&mut self, impact: ImpactResult) {
        self.impacts.push(impact);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.impacts.is_empty() {
            self.selected = (self.selected + 1).min(self.impacts.len() - 1);
        }
    }
}

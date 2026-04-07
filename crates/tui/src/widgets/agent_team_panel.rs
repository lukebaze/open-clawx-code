//! Agent team panel — shows multi-agent status in the context panel.
//!
//! Displays each agent's name, status badge, current task, and message count.
//! Rendered as a tab or floating panel in the TUI.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// View data for a single agent (decoupled from orchestrator types).
#[derive(Debug, Clone)]
pub struct AgentView {
    pub id: String,
    pub name: String,
    pub status_label: String,
    pub current_task: Option<String>,
    pub messages_sent: usize,
}

/// Agent team panel state.
pub struct AgentTeamPanel {
    pub agents: Vec<AgentView>,
    pub selected: usize,
}

impl Default for AgentTeamPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentTeamPanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            selected: 0,
        }
    }

    /// Update agent list from team state.
    pub fn set_agents(&mut self, agents: Vec<AgentView>) {
        self.agents = agents;
        if self.selected >= self.agents.len() {
            self.selected = self.agents.len().saturating_sub(1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.agents.is_empty() {
            self.selected = (self.selected + 1) % self.agents.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.agents.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.agents.len() - 1);
        }
    }

    /// Get the currently selected agent's ID.
    pub fn selected_agent_id(&self) -> Option<&str> {
        self.agents.get(self.selected).map(|a| a.id.as_str())
    }

    /// Render agent team content into a buffer area.
    pub fn render_content(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if self.agents.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No active agent team",
                    Style::new().fg(theme.dim),
                )),
                Line::from(Span::styled(
                    "  Use /team to start",
                    Style::new().fg(theme.dim),
                )),
            ];
            Paragraph::new(text).render(area, buf);
            return;
        }

        let lines: Vec<Line<'_>> = self
            .agents
            .iter()
            .enumerate()
            .flat_map(|(i, agent)| {
                let is_selected = i == self.selected;
                let prefix = if is_selected { "▸ " } else { "  " };
                let name_color = if is_selected { theme.accent } else { theme.fg };

                let status_color = match agent.status_label.as_str() {
                    "WORKING" => theme.warning,
                    "DONE" => theme.success,
                    "FAILED" => theme.error,
                    "WAITING" => theme.dim,
                    _ => theme.fg,
                };

                let mut result = vec![Line::from(vec![
                    Span::styled(prefix, Style::new().fg(name_color)),
                    Span::styled(&agent.name, Style::new().fg(name_color)),
                    Span::styled(" [", Style::new().fg(theme.dim)),
                    Span::styled(&agent.status_label, Style::new().fg(status_color)),
                    Span::styled(
                        format!("] msgs:{}", agent.messages_sent),
                        Style::new().fg(theme.dim),
                    ),
                ])];

                if let Some(task) = &agent.current_task {
                    let truncated = if task.len() > 40 {
                        format!("{}…", &task[..39])
                    } else {
                        task.clone()
                    };
                    result.push(Line::from(Span::styled(
                        format!("    {truncated}"),
                        Style::new().fg(theme.dim),
                    )));
                }

                result
            })
            .collect();

        Paragraph::new(lines).render(area, buf);
    }
}

impl Widget for &AgentTeamPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = Theme::default();
        self.render_content(area, buf, &theme);
    }
}

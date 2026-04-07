use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::input::InputMode;
use crate::modes::AgentMode;
use crate::theme::Theme;

/// Top status bar showing model, mode, session info, tokens, cost, and TDD state.
pub struct StatusBar<'a> {
    theme: &'a Theme,
    input_mode: InputMode,
    agent_mode: AgentMode,
    pending_key: bool,
    model: &'a str,
    total_tokens: u32,
    total_cost_usd: f64,
    session_name: &'a str,
    is_streaming: bool,
    /// TDD phase label (e.g., "RED", "GREEN", "E2E") — only in Build mode.
    tdd_phase: Option<&'a str>,
    /// Orchestrator iteration "current/max" — only in Build mode.
    iteration: Option<(u32, u32)>,
    /// Test result summary (e.g., "14/14") — only after tests run.
    test_summary: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        theme: &'a Theme,
        input_mode: InputMode,
        agent_mode: AgentMode,
        pending_key: bool,
        model: &'a str,
        total_tokens: u32,
        total_cost_usd: f64,
        session_name: &'a str,
        is_streaming: bool,
        tdd_phase: Option<&'a str>,
        iteration: Option<(u32, u32)>,
        test_summary: Option<&'a str>,
    ) -> Self {
        Self {
            theme,
            input_mode,
            agent_mode,
            pending_key,
            model,
            total_tokens,
            total_cost_usd,
            session_name,
            is_streaming,
            tdd_phase,
            iteration,
            test_summary,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::new().bg(self.theme.status_bg));

        // Input mode badge style
        let input_style = match self.input_mode {
            InputMode::Normal => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(100, 220, 140)),
            InputMode::Insert | InputMode::SlashComplete => {
                Style::new().fg(self.theme.bg).bg(self.theme.accent)
            }
            InputMode::Help => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(255, 200, 100)),
        };

        // Agent mode badge style
        let mode_style = match self.agent_mode {
            AgentMode::Plan => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(130, 170, 255)),
            AgentMode::Build => Style::new()
                .fg(self.theme.bg)
                .bg(ratatui::style::Color::Rgb(100, 220, 140)),
        };

        let dim = Style::new().fg(self.theme.dim).bg(self.theme.status_bg);

        // Left side: app name + model + mode badge + TDD info
        let mode_label = if self.agent_mode == AgentMode::Build {
            if let (Some(phase), Some((cur, max))) = (self.tdd_phase, self.iteration) {
                format!(" Build: {phase} {cur}/{max} ")
            } else {
                format!(" {} ", self.agent_mode.label())
            }
        } else {
            format!(" {} ", self.agent_mode.label())
        };

        let mut left_spans = vec![
            Span::styled(
                " OCX ",
                Style::new().fg(self.theme.accent).bg(self.theme.status_bg),
            ),
            Span::styled(format!(" {} ", self.model), dim),
            Span::styled(mode_label, mode_style),
        ];

        // Test summary badge (if available)
        if let Some(tests) = self.test_summary {
            left_spans.push(Span::styled(format!(" [{tests}] "), dim));
        }

        if self.pending_key {
            left_spans.push(Span::styled(" g-", dim));
        }

        // Right side: tokens, cost, input mode, streaming
        let tokens_str = format_tokens(self.total_tokens);
        let cost_str = format!("${:.2}", self.total_cost_usd);
        let streaming_indicator = if self.is_streaming { " ● " } else { "" };

        let right_text = format!(
            "{tokens_str} | {cost_str} | {} {streaming_indicator}",
            self.input_mode.label(),
        );

        // Calculate right-side start position
        let left_line = Line::from(left_spans.clone());
        #[allow(clippy::cast_possible_truncation)]
        let left_width = left_line.width() as u16;
        #[allow(clippy::cast_possible_truncation)]
        let right_width = right_text.len() as u16;

        // Render left side
        buf.set_line(area.x, area.y, &left_line, area.width);

        // Center: session name (if space permits)
        let center_start = left_width + 1;
        let center_end = area.width.saturating_sub(right_width + 1);
        if center_end > center_start + 4 {
            let max_name_len = (center_end - center_start) as usize;
            let name = if self.session_name.len() > max_name_len {
                &self.session_name[..max_name_len]
            } else {
                self.session_name
            };
            #[allow(clippy::cast_possible_truncation)]
            let name_len = name.len() as u16;
            let center_x = center_start + (center_end - center_start - name_len) / 2;
            let center_line = Line::from(Span::styled(name, dim));
            buf.set_line(
                area.x + center_x,
                area.y,
                &center_line,
                area.width - center_x,
            );
        }

        // Render right side
        let right_x = area.width.saturating_sub(right_width + 1);
        let mut right_spans = vec![
            Span::styled(tokens_str, dim),
            Span::styled(" | ", dim),
            Span::styled(cost_str, dim),
            Span::styled(" | ", dim),
            Span::styled(format!(" {} ", self.input_mode.label()), input_style),
        ];
        if self.is_streaming {
            right_spans.push(Span::styled(
                " ● ",
                Style::new()
                    .fg(ratatui::style::Color::Rgb(255, 200, 100))
                    .bg(self.theme.status_bg),
            ));
        }
        let right_line = Line::from(right_spans);
        buf.set_line(area.x + right_x, area.y, &right_line, right_width + 1);
    }
}

/// Format token count with K/M suffixes.
fn format_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", f64::from(tokens) / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", f64::from(tokens) / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

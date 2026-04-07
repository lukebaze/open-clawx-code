//! Diagnostics tab — shows LSP errors/warnings in the context panel.
//!
//! Displays diagnostics from language servers with severity filtering.
//! Part of the right-side context panel tab bar.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::Theme;

/// Filter for diagnostic severity display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFilter {
    All,
    ErrorOnly,
    WarningOnly,
}

impl DiagnosticFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ErrorOnly => "Errors",
            Self::WarningOnly => "Warnings",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::All => Self::ErrorOnly,
            Self::ErrorOnly => Self::WarningOnly,
            Self::WarningOnly => Self::All,
        }
    }
}

/// A single diagnostic entry for TUI display.
#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub file: String,
    pub line: u32,
    pub severity: String,
    pub message: String,
}

/// Diagnostics tab state.
pub struct DiagnosticsTab {
    pub entries: Vec<DiagnosticEntry>,
    pub filter: DiagnosticFilter,
    pub scroll_offset: usize,
}

impl Default for DiagnosticsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsTab {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filter: DiagnosticFilter::All,
            scroll_offset: 0,
        }
    }

    /// Replace all diagnostics with new entries.
    pub fn set_diagnostics(&mut self, entries: Vec<DiagnosticEntry>) {
        self.entries = entries;
        self.scroll_offset = 0;
    }

    /// Count of diagnostics matching current filter.
    pub fn visible_count(&self) -> usize {
        self.filtered_entries().count()
    }

    pub fn scroll_down(&mut self) {
        let max = self.visible_count().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + 1).min(max);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn filtered_entries(&self) -> impl Iterator<Item = &DiagnosticEntry> {
        self.entries.iter().filter(move |e| match self.filter {
            DiagnosticFilter::All => true,
            DiagnosticFilter::ErrorOnly => e.severity == "ERROR",
            DiagnosticFilter::WarningOnly => e.severity == "WARN",
        })
    }

    /// Render diagnostics content into a buffer area.
    pub fn render_content(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if self.entries.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No diagnostics",
                    Style::new().fg(theme.dim),
                )),
                Line::from(Span::styled(
                    "  LSP servers report here",
                    Style::new().fg(theme.dim),
                )),
            ];
            Paragraph::new(text).render(area, buf);
            return;
        }

        let lines: Vec<Line<'_>> = self
            .filtered_entries()
            .skip(self.scroll_offset)
            .take(area.height as usize)
            .map(|entry| {
                let severity_color = match entry.severity.as_str() {
                    "ERROR" => theme.error,
                    "WARN" => theme.warning,
                    _ => theme.dim,
                };
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", entry.severity),
                        Style::new().fg(severity_color),
                    ),
                    Span::styled(
                        format!("{}:{} ", entry.file, entry.line),
                        Style::new().fg(theme.dim),
                    ),
                    Span::styled(&entry.message, Style::new().fg(theme.fg)),
                ])
            })
            .collect();

        Paragraph::new(lines).render(area, buf);
    }
}

use ratatui::widgets::Widget;

impl Widget for &DiagnosticsTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Standalone render uses a default theme — normally called via render_content
        let theme = Theme::default();
        self.render_content(area, buf, &theme);
    }
}

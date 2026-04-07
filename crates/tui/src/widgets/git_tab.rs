use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// Parsed git status line
#[derive(Debug, Clone)]
pub struct GitStatusLine {
    pub status: String,
    pub path: String,
}

/// Git status view in the context panel
pub struct GitTab {
    status_lines: Vec<GitStatusLine>,
    dirty: bool,
}

impl GitTab {
    pub fn new() -> Self {
        Self {
            status_lines: Vec::new(),
            dirty: true,
        }
    }

    /// Mark that files may have changed and status needs refresh
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Refresh git status by running `git status --porcelain`
    pub fn refresh(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;

        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .output();

        self.status_lines = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|line| {
                        let (status, path) = if line.len() > 3 {
                            (line[..2].to_string(), line[3..].to_string())
                        } else {
                            (line.to_string(), String::new())
                        };
                        GitStatusLine { status, path }
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        };
    }

    pub fn render_content(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if self.status_lines.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Working tree clean",
                    Style::new().fg(theme.dim),
                )),
            ];
            Paragraph::new(text).render(area, buf);
            return;
        }

        let lines: Vec<Line<'_>> = self
            .status_lines
            .iter()
            .map(|entry| {
                let color = match entry.status.trim() {
                    s if s.starts_with('M') || s.starts_with('A') => {
                        ratatui::style::Color::Rgb(100, 220, 140) // green = staged
                    }
                    s if s.ends_with('M') || s.ends_with('D') => {
                        ratatui::style::Color::Rgb(255, 100, 100) // red = unstaged
                    }
                    "??" => theme.dim, // untracked
                    _ => theme.fg,
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", entry.status), Style::new().fg(color)),
                    Span::styled(&entry.path, Style::new().fg(theme.fg)),
                ])
            })
            .collect();

        Paragraph::new(lines).render(area, buf);
    }
}

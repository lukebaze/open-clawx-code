use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// How a file was referenced in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileSource {
    Read,
    Modified,
    Mentioned,
}

/// A file entry tracked in the files tab
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub source: FileSource,
}

/// Tracks files referenced during the conversation
pub struct FilesTab {
    entries: Vec<FileEntry>,
}

impl FilesTab {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add(&mut self, path: String, source: FileSource) {
        // Update existing entry if present (upgrade source priority)
        if let Some(existing) = self.entries.iter_mut().find(|e| e.path == path) {
            if source == FileSource::Modified {
                existing.source = FileSource::Modified;
            }
            return;
        }
        self.entries.push(FileEntry { path, source });
    }

    pub fn render_content(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if self.entries.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No files referenced yet",
                    Style::new().fg(theme.dim),
                )),
            ];
            Paragraph::new(text).render(area, buf);
            return;
        }

        let lines: Vec<Line<'_>> = self
            .entries
            .iter()
            .map(|entry| {
                let icon = match entry.source {
                    FileSource::Read => "R",
                    FileSource::Modified => "M",
                    FileSource::Mentioned => " ",
                };
                let icon_color = match entry.source {
                    FileSource::Read => theme.fg,
                    FileSource::Modified => ratatui::style::Color::Rgb(255, 200, 100),
                    FileSource::Mentioned => theme.dim,
                };
                Line::from(vec![
                    Span::styled(format!(" {icon} "), Style::new().fg(icon_color)),
                    Span::styled(&entry.path, Style::new().fg(theme.fg)),
                ])
            })
            .collect();

        Paragraph::new(lines).render(area, buf);
    }
}

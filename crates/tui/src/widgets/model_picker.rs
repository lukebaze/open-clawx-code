//! Model picker dialog — lets users switch LLM models and providers.
//!
//! Shown via `/model` command. Lists all available models from all
//! registered providers with selection and keyboard navigation.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::theme::Theme;

/// A model entry for display in the picker.
#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u32,
    pub is_active: bool,
}

/// Model picker overlay state.
pub struct ModelPickerState {
    pub models: Vec<ModelEntry>,
    pub selected: usize,
    pub visible: bool,
}

impl ModelPickerState {
    #[must_use]
    pub fn new(models: Vec<ModelEntry>) -> Self {
        let selected = models.iter().position(|m| m.is_active).unwrap_or(0);
        Self {
            models,
            selected,
            visible: true,
        }
    }

    pub fn move_up(&mut self) {
        if !self.models.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.models.len() - 1);
        }
    }

    pub fn move_down(&mut self) {
        if !self.models.is_empty() {
            self.selected = (self.selected + 1) % self.models.len();
        }
    }

    #[must_use]
    pub fn selected_model_id(&self) -> Option<&str> {
        self.models.get(self.selected).map(|m| m.id.as_str())
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

/// Model picker overlay widget.
pub struct ModelPicker<'a> {
    pub models: &'a [ModelEntry],
    pub selected: usize,
    pub theme: &'a Theme,
}

impl Widget for ModelPicker<'_> {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = (area.width * 60 / 100).max(40).min(area.width - 4);
        let count = self.models.len().min(u16::MAX as usize) as u16;
        let height = (count + 4).min(20).min(area.height - 2);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Select Model (j/k Enter Esc) ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.theme.accent))
            .style(Style::new().bg(self.theme.bg));

        let inner = block.inner(popup);
        block.render(popup, buf);

        let lines: Vec<Line<'_>> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, model)| {
                let is_sel = i == self.selected;
                let prefix = if model.is_active {
                    "● "
                } else if is_sel {
                    "▸ "
                } else {
                    "  "
                };
                let name_color = if is_sel { self.theme.accent } else { self.theme.fg };
                let ctx_k = model.context_window / 1000;
                Line::from(vec![
                    Span::styled(prefix, Style::new().fg(self.theme.accent)),
                    Span::styled(&model.name, Style::new().fg(name_color)),
                    Span::styled(
                        format!("  ({}, {ctx_k}k)", model.provider),
                        Style::new().fg(self.theme.dim),
                    ),
                ])
            })
            .collect();

        Paragraph::new(lines).render(inner, buf);
    }
}

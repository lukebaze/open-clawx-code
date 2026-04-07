//! Config editor overlay — edit API keys and settings in-TUI.
//!
//! Shown via `/config` command. Displays provider list with masked keys,
//! allows inline editing with Enter to reveal/edit, Tab to cycle fields.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::config::UserConfig;
use crate::theme::Theme;

/// A single editable field in the config editor.
#[derive(Debug, Clone)]
pub struct ConfigField {
    pub provider: String,
    pub env_var: String,
    pub value: String,
    pub editing: bool,
}

impl ConfigField {
    /// Display value: masked unless currently editing.
    #[must_use]
    pub fn display_value(&self) -> String {
        if self.editing {
            return self.value.clone();
        }
        if self.value.is_empty() {
            return "(not set)".to_string();
        }
        // Show first 6 + last 4 chars, mask middle
        if self.value.len() > 12 {
            let prefix = &self.value[..6];
            let suffix = &self.value[self.value.len() - 4..];
            format!("{prefix}...{suffix}")
        } else {
            "****".to_string()
        }
    }
}

/// Config editor overlay state.
pub struct ConfigEditorState {
    pub fields: Vec<ConfigField>,
    pub selected: usize,
    pub visible: bool,
    pub dirty: bool,
}

impl ConfigEditorState {
    /// Create from current user config.
    #[must_use]
    pub fn from_config(config: &UserConfig) -> Self {
        let fields = UserConfig::known_providers()
            .into_iter()
            .filter(|(name, _)| *name != "ollama") // no key needed
            .map(|(name, env_var)| ConfigField {
                provider: name.to_string(),
                env_var: env_var.to_string(),
                value: config.get_key(name).unwrap_or("").to_string(),
                editing: false,
            })
            .collect();
        Self {
            fields,
            selected: 0,
            visible: true,
            dirty: false,
        }
    }

    pub fn move_up(&mut self) {
        self.stop_editing();
        if !self.fields.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.fields.len() - 1);
        }
    }

    pub fn move_down(&mut self) {
        self.stop_editing();
        if !self.fields.is_empty() {
            self.selected = (self.selected + 1) % self.fields.len();
        }
    }

    pub fn start_editing(&mut self) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            field.editing = true;
        }
    }

    pub fn stop_editing(&mut self) {
        for field in &mut self.fields {
            field.editing = false;
        }
    }

    pub fn is_editing(&self) -> bool {
        self.fields.iter().any(|f| f.editing)
    }

    pub fn type_char(&mut self, ch: char) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            if field.editing {
                field.value.push(ch);
                self.dirty = true;
            }
        }
    }

    pub fn backspace(&mut self) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            if field.editing {
                field.value.pop();
                self.dirty = true;
            }
        }
    }

    /// Apply edited values back to a `UserConfig`.
    pub fn apply_to(&self, config: &mut UserConfig) {
        for field in &self.fields {
            config.set_key(&field.provider, field.value.clone());
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

/// Config editor overlay widget.
pub struct ConfigEditor<'a> {
    pub fields: &'a [ConfigField],
    pub selected: usize,
    pub theme: &'a Theme,
}

impl Widget for ConfigEditor<'_> {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = (area.width * 70 / 100).max(50).min(area.width - 4);
        let count = self.fields.len().min(u16::MAX as usize) as u16;
        let height = (count * 2 + 5).min(area.height - 2);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        Clear.render(popup, buf);

        let block = Block::default()
            .title(" API Keys Configuration (j/k Enter Esc) ")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.theme.accent))
            .style(Style::new().bg(self.theme.bg));

        let inner = block.inner(popup);
        block.render(popup, buf);

        let mut lines: Vec<Line<'_>> = Vec::new();
        lines.push(Line::from(Span::styled(
            " Set API keys for each provider. Enter=edit, Esc=save & close",
            Style::new().fg(self.theme.dim),
        )));
        lines.push(Line::from(""));

        for (i, field) in self.fields.iter().enumerate() {
            let is_sel = i == self.selected;
            let prefix = if is_sel { "▸ " } else { "  " };
            let label_color = if is_sel { self.theme.accent } else { self.theme.fg };
            let val_display = field.display_value();
            let val_color = if field.editing {
                self.theme.warning
            } else if field.value.is_empty() {
                self.theme.error
            } else {
                self.theme.success
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, Style::new().fg(self.theme.accent)),
                Span::styled(
                    format!("{:<12}", field.provider),
                    Style::new().fg(label_color),
                ),
                Span::styled(val_display, Style::new().fg(val_color)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("    env: {}", field.env_var),
                Style::new().fg(self.theme.dim),
            )));
        }

        Paragraph::new(lines).render(inner, buf);
    }
}

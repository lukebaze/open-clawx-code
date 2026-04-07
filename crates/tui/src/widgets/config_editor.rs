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

/// What kind of field this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    ApiKey,
    BaseUrl,
    Models,
    /// Separator label (not editable).
    Label,
}

/// A single editable field in the config editor.
#[derive(Debug, Clone)]
pub struct ConfigField {
    pub provider: String,
    pub label: String,
    pub value: String,
    pub kind: FieldKind,
    pub editing: bool,
}

impl ConfigField {
    /// Display value: masked for API keys unless editing.
    #[must_use]
    pub fn display_value(&self) -> String {
        if self.editing {
            return self.value.clone();
        }
        if self.value.is_empty() {
            return "(not set)".to_string();
        }
        if self.kind == FieldKind::ApiKey && self.value.len() > 12 {
            let prefix = &self.value[..6];
            let suffix = &self.value[self.value.len() - 4..];
            return format!("{prefix}...{suffix}");
        }
        if self.kind == FieldKind::ApiKey {
            return "****".to_string();
        }
        self.value.clone()
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
        let mut fields: Vec<ConfigField> = Vec::new();

        // Built-in providers
        fields.push(ConfigField {
            provider: String::new(),
            label: "── Built-in Providers ──".to_string(),
            value: String::new(),
            kind: FieldKind::Label,
            editing: false,
        });
        for (name, env_var) in UserConfig::known_providers() {
            if name == "ollama" {
                continue;
            }
            fields.push(ConfigField {
                provider: name.to_string(),
                label: format!("{name} ({env_var})"),
                value: config.get_key(name).unwrap_or("").to_string(),
                kind: FieldKind::ApiKey,
                editing: false,
            });
        }

        // Custom providers
        if !config.custom_providers.is_empty() {
            fields.push(ConfigField {
                provider: String::new(),
                label: "── Custom Providers ──".to_string(),
                value: String::new(),
                kind: FieldKind::Label,
                editing: false,
            });
            for cp in &config.custom_providers {
                fields.push(ConfigField {
                    provider: cp.name.clone(),
                    label: format!("{} base_url", cp.name),
                    value: cp.base_url.clone(),
                    kind: FieldKind::BaseUrl,
                    editing: false,
                });
                fields.push(ConfigField {
                    provider: cp.name.clone(),
                    label: format!("{} api_key", cp.name),
                    value: cp.api_key.clone(),
                    kind: FieldKind::ApiKey,
                    editing: false,
                });
                fields.push(ConfigField {
                    provider: cp.name.clone(),
                    label: format!("{} models", cp.name),
                    value: cp.models.join(", "),
                    kind: FieldKind::Models,
                    editing: false,
                });
            }
        }

        // "Add custom" pseudo-field
        fields.push(ConfigField {
            provider: String::new(),
            label: "[+ Add custom provider]".to_string(),
            value: String::new(),
            kind: FieldKind::Label,
            editing: false,
        });

        Self {
            fields,
            selected: 1, // skip first label
            visible: true,
            dirty: false,
        }
    }

    pub fn move_up(&mut self) {
        self.stop_editing();
        if self.fields.is_empty() {
            return;
        }
        // Skip label fields
        let mut idx = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.fields.len() - 1);
        while idx != self.selected && self.fields[idx].kind == FieldKind::Label {
            idx = idx.checked_sub(1).unwrap_or(self.fields.len() - 1);
        }
        self.selected = idx;
    }

    pub fn move_down(&mut self) {
        self.stop_editing();
        if self.fields.is_empty() {
            return;
        }
        let mut idx = (self.selected + 1) % self.fields.len();
        while idx != self.selected && self.fields[idx].kind == FieldKind::Label {
            idx = (idx + 1) % self.fields.len();
        }
        self.selected = idx;
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
        use crate::config::CustomProvider;
        use std::collections::BTreeMap;

        // Collect custom providers by name
        let mut custom_map: BTreeMap<String, CustomProvider> = BTreeMap::new();

        for field in &self.fields {
            if field.kind == FieldKind::Label || field.provider.is_empty() {
                continue;
            }
            match field.kind {
                FieldKind::ApiKey if !custom_map.contains_key(&field.provider) => {
                    // Built-in provider key
                    config.set_key(&field.provider, field.value.clone());
                }
                FieldKind::BaseUrl => {
                    custom_map
                        .entry(field.provider.clone())
                        .or_insert_with(|| CustomProvider {
                            name: field.provider.clone(),
                            base_url: String::new(),
                            api_key: String::new(),
                            models: Vec::new(),
                        })
                        .base_url
                        .clone_from(&field.value);
                }
                FieldKind::ApiKey => {
                    custom_map
                        .entry(field.provider.clone())
                        .or_insert_with(|| CustomProvider {
                            name: field.provider.clone(),
                            base_url: String::new(),
                            api_key: String::new(),
                            models: Vec::new(),
                        })
                        .api_key
                        .clone_from(&field.value);
                }
                FieldKind::Models => {
                    let models: Vec<String> = field
                        .value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    custom_map
                        .entry(field.provider.clone())
                        .or_insert_with(|| CustomProvider {
                            name: field.provider.clone(),
                            base_url: String::new(),
                            api_key: String::new(),
                            models: Vec::new(),
                        })
                        .models = models;
                }
                FieldKind::Label => {}
            }
        }

        config.custom_providers = custom_map.into_values().collect();
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
            " Enter=edit, Esc=save & close, /config add <name>=add custom",
            Style::new().fg(self.theme.dim),
        )));
        lines.push(Line::from(""));

        for (i, field) in self.fields.iter().enumerate() {
            let is_sel = i == self.selected;

            // Labels are non-editable separators
            if field.kind == FieldKind::Label {
                lines.push(Line::from(Span::styled(
                    format!("  {}", field.label),
                    Style::new().fg(self.theme.accent),
                )));
                continue;
            }

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
                    format!("{:<20}", field.label),
                    Style::new().fg(label_color),
                ),
                Span::styled(val_display, Style::new().fg(val_color)),
            ]));
        }

        Paragraph::new(lines).render(inner, buf);
    }
}

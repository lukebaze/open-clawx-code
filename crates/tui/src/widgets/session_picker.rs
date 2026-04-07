use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::session_manager::{format_relative_time, SessionSummary};
use crate::theme::Theme;

/// Modal overlay listing recent sessions for selection.
pub struct SessionPicker<'a> {
    pub sessions: &'a [SessionSummary],
    pub selected: usize,
    pub theme: &'a Theme,
}

impl Widget for SessionPicker<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Center a box ~60 wide, height = min(sessions + 4, area.height - 4)
        let width = 60.min(area.width.saturating_sub(4));
        #[allow(clippy::cast_possible_truncation)]
        let session_count = self.sessions.len().min(u16::MAX as usize) as u16;
        let height = (session_count + 4)
            .min(area.height.saturating_sub(4))
            .max(6);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog = Rect::new(x, y, width, height);

        // Clear background
        Clear.render(dialog, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.theme.accent))
            .title(" Sessions (↑↓ select, Enter load, n new) ");
        let inner = block.inner(dialog);
        block.render(dialog, buf);

        if self.sessions.is_empty() {
            let line = Line::from(Span::styled(
                "  No previous sessions found. Press 'n' to create one.",
                Style::new().fg(self.theme.dim),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        // Render session rows
        let visible_count = inner.height as usize;
        let start = if self.selected >= visible_count {
            self.selected - visible_count + 1
        } else {
            0
        };

        for (i, session) in self
            .sessions
            .iter()
            .skip(start)
            .take(visible_count)
            .enumerate()
        {
            let is_selected = (start + i) == self.selected;
            let style = if is_selected {
                Style::new()
                    .fg(self.theme.bg)
                    .bg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(self.theme.fg)
            };

            let time_str = format_relative_time(session.modified_epoch_millis);
            let msg_str = format!("{}msg", session.message_count);
            // Truncate title to fit
            let max_title =
                (inner.width as usize).saturating_sub(time_str.len() + msg_str.len() + 6);
            let title = if session.title.len() > max_title {
                format!("{}…", &session.title[..max_title.saturating_sub(1)])
            } else {
                session.title.clone()
            };

            let line = Line::from(vec![
                Span::styled(if is_selected { " ▸ " } else { "   " }, style),
                Span::styled(title, style),
                Span::styled(
                    format!(
                        "{:>width$}",
                        format!("{msg_str}  {time_str}"),
                        width = (inner.width as usize).saturating_sub(max_title + 4)
                    ),
                    if is_selected {
                        style
                    } else {
                        Style::new().fg(self.theme.dim)
                    },
                ),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            let row_y = inner.y + (i as u16);
            buf.set_line(inner.x, row_y, &line, inner.width);
        }
    }
}

/// State for the session picker interaction.
pub struct SessionPickerState {
    pub sessions: Vec<SessionSummary>,
    pub selected: usize,
    pub visible: bool,
}

impl SessionPickerState {
    pub fn new(sessions: Vec<SessionSummary>) -> Self {
        let visible = !sessions.is_empty();
        Self {
            sessions,
            selected: 0,
            visible,
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1).min(self.sessions.len() - 1);
        }
    }

    /// Get the ID of the currently selected session.
    pub fn selected_id(&self) -> Option<&str> {
        self.sessions.get(self.selected).map(|s| s.id.as_str())
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

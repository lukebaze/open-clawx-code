use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Tabs, Widget},
};

use crate::theme::Theme;
use crate::widgets::files_tab::FilesTab;
use crate::widgets::git_tab::GitTab;

/// Active tab in the context panel
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContextTab {
    Files,
    Git,
    GitNexus,
}

impl ContextTab {
    pub fn next(self) -> Self {
        match self {
            Self::Files => Self::Git,
            Self::Git => Self::GitNexus,
            Self::GitNexus => Self::Files,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Files => 0,
            Self::Git => 1,
            Self::GitNexus => 2,
        }
    }
}

/// Right-side context panel with tabbed views
pub struct ContextPanelWidget<'a> {
    pub active_tab: ContextTab,
    pub files_tab: &'a FilesTab,
    pub git_tab: &'a GitTab,
    pub theme: &'a Theme,
    pub focused: bool,
}

impl Widget for ContextPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(self.theme.bg));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 {
            return;
        }

        // Split inner into tab bar (1 row) + content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(inner);

        // Tab bar
        let tab_titles = vec![
            Span::styled(" Files ", Style::new().fg(self.theme.fg)),
            Span::styled(" Git ", Style::new().fg(self.theme.fg)),
            Span::styled(" GitNexus ", Style::new().fg(self.theme.fg)),
        ];
        let tabs = Tabs::new(tab_titles)
            .select(self.active_tab.index())
            .highlight_style(Style::new().fg(self.theme.accent));
        tabs.render(chunks[0], buf);

        // Tab content
        match self.active_tab {
            ContextTab::Files => self.files_tab.render_content(chunks[1], buf, self.theme),
            ContextTab::Git => self.git_tab.render_content(chunks[1], buf, self.theme),
            ContextTab::GitNexus => render_gitnexus_placeholder(chunks[1], buf, self.theme),
        }
    }
}

fn render_gitnexus_placeholder(area: Rect, buf: &mut Buffer, theme: &Theme) {
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  GitNexus integration",
            Style::new().fg(theme.accent),
        )),
        Line::from(Span::styled(
            "  Coming in Phase 06",
            Style::new().fg(theme.dim),
        )),
    ];
    Paragraph::new(text).render(area, buf);
}

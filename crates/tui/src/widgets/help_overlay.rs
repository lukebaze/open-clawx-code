use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::theme::Theme;

/// Full-screen keybinding help overlay
pub struct HelpOverlay<'a> {
    pub theme: &'a Theme,
}

impl Widget for HelpOverlay<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Centered overlay (80% width, 80% height)
        let w = (area.width * 4 / 5).min(70);
        let h = (area.height * 4 / 5).min(30);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let overlay = Rect::new(x, y, w, h);

        Clear.render(overlay, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.theme.accent))
            .style(Style::new().bg(self.theme.bg))
            .title(" Keybindings — press any key to close ");

        let heading = Style::new().fg(self.theme.accent);
        let key_style = Style::new().fg(self.theme.fg);
        let desc_style = Style::new().fg(self.theme.dim);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(" GLOBAL", heading)),
            binding("  Ctrl+C", "Quit / Cancel generation", key_style, desc_style),
            binding("  Ctrl+H", "Focus conversation panel", key_style, desc_style),
            binding("  Ctrl+L", "Focus context panel", key_style, desc_style),
            binding("  Ctrl+P", "Command palette (coming soon)", key_style, desc_style),
            binding("  Ctrl+[/]", "Resize right panel", key_style, desc_style),
            binding("  Esc", "Switch to Normal mode", key_style, desc_style),
            Line::from(""),
            Line::from(Span::styled(" NORMAL MODE", heading)),
            binding("  j / k", "Scroll conversation down / up", key_style, desc_style),
            binding("  G", "Scroll to bottom", key_style, desc_style),
            binding("  gg", "Scroll to top", key_style, desc_style),
            binding("  i", "Switch to Insert mode", key_style, desc_style),
            binding("  q", "Quit", key_style, desc_style),
            binding("  ?", "Show this help", key_style, desc_style),
            binding("  Tab", "Cycle panel focus", key_style, desc_style),
            binding("  1/2/3", "Switch context tab (when focused)", key_style, desc_style),
            Line::from(""),
            Line::from(Span::styled(" INSERT MODE", heading)),
            binding("  Enter", "Send message", key_style, desc_style),
            binding("  /", "Slash command autocomplete", key_style, desc_style),
            binding("  Tab", "Select autocomplete / cycle focus", key_style, desc_style),
        ];

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        paragraph.render(overlay, buf);
    }
}

fn binding<'a>(key: &'a str, desc: &'a str, ks: Style, ds: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<14}"), ks),
        Span::styled(desc, ds),
    ])
}

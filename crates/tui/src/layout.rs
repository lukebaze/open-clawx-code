use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Splits the terminal into: status bar (1 row), main area (fill), input bar (3 rows).
/// Main area is split 70/30 into conversation (left) and context panel (right).
pub struct AppLayout {
    pub status_bar: Rect,
    pub conversation: Rect,
    pub context_panel: Rect,
    pub input_bar: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // status bar
                Constraint::Fill(1),   // main area
                Constraint::Length(3), // input bar
            ])
            .split(area);

        let main_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // conversation
                Constraint::Percentage(30), // context panel
            ])
            .split(vertical[1]);

        Self {
            status_bar: vertical[0],
            conversation: main_cols[0],
            context_panel: main_cols[1],
            input_bar: vertical[2],
        }
    }
}

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Splits the terminal into: status bar (1 row), main area (fill), input bar (3 rows).
/// Main area is split into conversation (left) and context panel (right).
pub struct AppLayout {
    pub status_bar: Rect,
    pub conversation: Rect,
    pub context_panel: Rect,
    pub input_bar: Rect,
}

impl AppLayout {
    /// Build layout with configurable right panel percentage (20-50 range).
    pub fn new(area: Rect, right_panel_pct: u16) -> Self {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status bar
                Constraint::Fill(1),   // main area
                Constraint::Length(3), // input bar
            ])
            .split(area);

        let left_pct = 100 - right_panel_pct;
        let main_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(left_pct),
                Constraint::Percentage(right_panel_pct),
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

use ratatui::layout::{Constraint, Direction, Layout as RatatuiLayout, Rect};

/// Layout manager for the TUI
pub struct Layout;

impl Layout {
    /// Create the main layout with status bar, content area, and command bar
    ///
    /// Returns: (status_area, content_area, command_area)
    pub fn main(area: Rect) -> (Rect, Rect, Rect) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Status bar
                Constraint::Min(0),    // Content area
                Constraint::Length(1), // Command bar
            ])
            .split(area);

        (chunks[0], chunks[1], chunks[2])
    }

    /// Split content area into two panes (scratch left, board right)
    ///
    /// Returns: (scratch_area, board_area)
    pub fn panes(area: Rect) -> (Rect, Rect) {
        let chunks = RatatuiLayout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Scratch panel (left)
                Constraint::Percentage(50), // Board panel (right)
            ])
            .split(area);

        (chunks[0], chunks[1])
    }
}

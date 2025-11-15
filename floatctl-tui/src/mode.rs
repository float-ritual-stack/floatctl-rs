/// Application modes (vim-inspired)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    /// Navigate scratch + boards (vim normal mode)
    Normal,

    /// Edit scratch log (vim insert mode)
    Insert,

    /// Command input (vim : mode)
    Command,

    /// Board navigation (switch between boards)
    BoardNav,
}

impl AppMode {
    /// Get display name for status bar
    pub fn display_name(&self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Insert => "INSERT",
            AppMode::Command => "COMMAND",
            AppMode::BoardNav => "BOARD NAV",
        }
    }

    /// Get color for status bar (in ratatui Color enum)
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            AppMode::Normal => Color::Cyan,
            AppMode::Insert => Color::Green,
            AppMode::Command => Color::Yellow,
            AppMode::BoardNav => Color::Magenta,
        }
    }
}

/// Which pane has focus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    /// Left pane (scratch log)
    Scratch,

    /// Right pane (board view)
    Board,
}

impl Pane {
    /// Toggle between panes
    pub fn toggle(&self) -> Self {
        match self {
            Pane::Scratch => Pane::Board,
            Pane::Board => Pane::Scratch,
        }
    }
}

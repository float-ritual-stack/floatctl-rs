pub mod board_panel;
pub mod command_bar;
pub mod layout;
pub mod scratch_panel;
pub mod status_bar;

use ratatui::Frame;

use crate::app::App;
use crate::mode::{AppMode, Pane};

pub use board_panel::BoardPanel;
pub use scratch_panel::ScratchPanel;

/// Main UI renderer
pub struct UI<'a> {
    pub scratch_panel: ScratchPanel<'a>,
    pub board_panel: BoardPanel,
}

impl<'a> UI<'a> {
    /// Create a new UI
    pub fn new() -> Self {
        Self {
            scratch_panel: ScratchPanel::new(),
            board_panel: BoardPanel::new(),
        }
    }

    /// Render the entire UI
    pub fn render(&mut self, f: &mut Frame, app: &App) {
        // Get main layout areas
        let (status_area, content_area, command_area) = layout::Layout::main(f.area());

        // Render status bar
        status_bar::render(f, status_area, app);

        // Render command bar
        command_bar::render(f, command_area, app);

        // Split content into panes
        let (scratch_area, board_area) = layout::Layout::panes(content_area);

        // Render scratch panel
        self.scratch_panel.render(f, scratch_area, app);

        // Render board panel (stateless - pass state from App)
        self.board_panel.render(
            f,
            board_area,
            app,
            &app.board_blocks,
            app.board_selected,
        );
    }

    /// Handle input events (delegates to appropriate component)
    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent, app: &App) {
        // Only scratch panel handles input when in insert mode and focused
        if app.mode == AppMode::Insert && app.focused_pane == Pane::Scratch {
            self.scratch_panel.handle_input(key);
        }
    }
}

impl<'a> Default for UI<'a> {
    fn default() -> Self {
        Self::new()
    }
}

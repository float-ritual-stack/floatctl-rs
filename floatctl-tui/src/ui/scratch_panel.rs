use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::TextArea;

use crate::app::App;
use crate::mode::{AppMode, Pane};

/// Scratch panel state
pub struct ScratchPanel<'a> {
    pub textarea: TextArea<'a>,
}

impl<'a> ScratchPanel<'a> {
    /// Create a new scratch panel
    pub fn new() -> Self {
        let mut textarea = TextArea::default();

        // Set placeholder text
        textarea.set_placeholder_text("Type ctx:: to start a context entry...");

        // Set block style
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Scratch Log ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        Self { textarea }
    }

    /// Render the scratch panel
    pub fn render(&mut self, f: &mut Frame, area: Rect, app: &App) {
        // Update border color based on focus
        let border_color = if app.focused_pane == Pane::Scratch {
            app.mode.color()
        } else {
            Color::DarkGray
        };

        let title = if app.mode == AppMode::Insert && app.focused_pane == Pane::Scratch {
            " Scratch Log [EDITING] "
        } else {
            " Scratch Log "
        };

        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        );

        // Set cursor style based on mode
        if app.mode == AppMode::Insert && app.focused_pane == Pane::Scratch {
            self.textarea
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            self.textarea.set_cursor_style(Style::default());
        }

        f.render_widget(&self.textarea, area);
    }

    /// Handle key input (only when in insert mode and focused)
    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent) {
        self.textarea.input(key);
    }

    /// Get current content
    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Load content from string
    pub fn load_content(&mut self, content: &str) {
        // Create new textarea with content
        let mut new_textarea = TextArea::from(content.lines().map(|s| s.to_string()));

        // Copy over styling
        new_textarea.set_placeholder_text("Type ctx:: to start a context entry...");
        new_textarea.set_block(self.textarea.block().cloned().unwrap_or_default());
        new_textarea.set_cursor_style(self.textarea.cursor_style());

        self.textarea = new_textarea;
    }
}

impl<'a> Default for ScratchPanel<'a> {
    fn default() -> Self {
        Self::new()
    }
}

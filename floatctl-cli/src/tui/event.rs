//! Event handling for the TUI

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, FocusedPane, MainTab, Mode};
use super::commands::{self, Command, ParseResult};

/// Poll for events with timeout
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Handle a key event
pub fn handle_key(app: &mut App, key: KeyEvent) -> HandleResult {
    // Global quit shortcuts (Ctrl+C, Ctrl+Q)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return HandleResult::Quit,
            _ => {}
        }
    }

    // Mode-specific handling
    match app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Edit => handle_edit_mode(app, key),
        Mode::ActionPalette => handle_action_palette(app, key),
        Mode::Search => handle_search_mode(app, key),
    }
}

/// Result of handling a key event
pub enum HandleResult {
    /// Continue running
    Continue,
    /// Quit the application
    Quit,
    /// Execute an action by ID
    ExecuteAction(String),
    /// Navigate into item
    NavigateInto,
    /// Navigate back
    NavigateBack,
    /// Refresh current view
    Refresh,
    /// Execute a parsed command
    ExecuteCommand(Command),
    /// Update filter (re-filter displayed items)
    UpdateFilter,
    /// Switch to specific tab
    SwitchTab(MainTab),
    /// Show help
    ShowHelp,
}

/// Handle keys in normal mode
fn handle_normal_mode(app: &mut App, key: KeyEvent) -> HandleResult {
    match key.code {
        // Quit
        KeyCode::Char('q') => HandleResult::Quit,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            HandleResult::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev();
            HandleResult::Continue
        }

        // Enter - navigate into or open action palette
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
            if let Some(item) = app.selected_item() {
                if item.has_children {
                    HandleResult::NavigateInto
                } else {
                    app.open_action_palette();
                    HandleResult::Continue
                }
            } else {
                HandleResult::Continue
            }
        }

        // Back navigation
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
            HandleResult::NavigateBack
        }

        // Escape goes back or does nothing at root
        KeyCode::Esc => {
            if app.nav_stack.is_empty() && app.current_nav.is_none() {
                HandleResult::Continue
            } else {
                HandleResult::NavigateBack
            }
        }

        // Action palette
        KeyCode::Char('a') => {
            app.open_action_palette();
            HandleResult::Continue
        }

        // Search/filter
        KeyCode::Char('/') => {
            app.start_filter();
            HandleResult::Continue
        }

        // Help
        KeyCode::Char('?') => HandleResult::ShowHelp,

        // Tab switching
        KeyCode::Char('1') => {
            app.switch_tab(MainTab::Home);
            HandleResult::Refresh
        }
        KeyCode::Char('2') => {
            app.switch_tab(MainTab::Boards);
            HandleResult::Refresh
        }
        KeyCode::Char('3') => {
            app.switch_tab(MainTab::Search);
            HandleResult::Refresh
        }
        KeyCode::Char('4') => {
            app.switch_tab(MainTab::Dashboard);
            HandleResult::Refresh
        }

        // Pane toggle
        KeyCode::Tab => {
            app.toggle_pane();
            HandleResult::Continue
        }

        // Scratch pane edit
        KeyCode::Char('i') if app.focused_pane == FocusedPane::Main => {
            app.enter_scratch_edit();
            HandleResult::Continue
        }

        // Toggle preview
        KeyCode::Char('p') => {
            app.show_preview = !app.show_preview;
            HandleResult::Continue
        }

        // Refresh
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            HandleResult::Refresh
        }

        // Page navigation
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.select_next();
            }
            HandleResult::Continue
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.select_prev();
            }
            HandleResult::Continue
        }

        // Home/End
        KeyCode::Home | KeyCode::Char('g') => {
            app.selected_index = 0;
            app.scroll_offset = 0;
            HandleResult::Continue
        }
        KeyCode::End | KeyCode::Char('G') => {
            if !app.list_items.is_empty() {
                app.selected_index = app.list_items.len() - 1;
            }
            HandleResult::Continue
        }

        _ => HandleResult::Continue,
    }
}

/// Handle keys in edit mode (scratch pane)
fn handle_edit_mode(app: &mut App, key: KeyEvent) -> HandleResult {
    match key.code {
        KeyCode::Esc => {
            app.exit_mode();
            HandleResult::Continue
        }
        KeyCode::Enter => {
            // Parse scratch content for commands
            match commands::parse_scratch(&app.scratch_content) {
                ParseResult::Command(cmd) => {
                    // Clear scratch and execute
                    app.execute_scratch();
                    HandleResult::ExecuteCommand(cmd)
                }
                ParseResult::Text(_) | ParseResult::Incomplete => {
                    // Not a command, insert newline
                    app.scratch_insert('\n');
                    HandleResult::Continue
                }
            }
        }
        KeyCode::Backspace => {
            app.scratch_backspace();
            // Update completions
            update_completions(app);
            HandleResult::Continue
        }
        KeyCode::Char(c) => {
            app.scratch_insert(c);
            // Update completions
            update_completions(app);
            HandleResult::Continue
        }
        KeyCode::Tab => {
            // Tab completion
            if !app.completions.is_empty() {
                let completion = app.completions[0].clone();
                app.scratch_content = completion;
                app.scratch_cursor = app.scratch_content.len();
                app.completions.clear();
            }
            HandleResult::Continue
        }
        KeyCode::Left => {
            if app.scratch_cursor > 0 {
                app.scratch_cursor -= 1;
            }
            HandleResult::Continue
        }
        KeyCode::Right => {
            if app.scratch_cursor < app.scratch_content.len() {
                app.scratch_cursor += 1;
            }
            HandleResult::Continue
        }
        _ => HandleResult::Continue,
    }
}

/// Update completions based on scratch content
fn update_completions(app: &mut App) {
    let content = app.scratch_content.trim();
    if content.starts_with('/') {
        app.completions = commands::get_completions(content)
            .into_iter()
            .map(String::from)
            .collect();
    } else {
        app.completions.clear();
    }
}

/// Handle keys in action palette mode
fn handle_action_palette(app: &mut App, key: KeyEvent) -> HandleResult {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_action_palette();
            HandleResult::Continue
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            HandleResult::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev();
            HandleResult::Continue
        }
        KeyCode::Enter => {
            if let Some(action) = app.action_items.get(app.action_selected) {
                let action_id = action.id.clone();
                app.close_action_palette();
                HandleResult::ExecuteAction(action_id)
            } else {
                HandleResult::Continue
            }
        }
        // Quick action shortcuts (1-9)
        KeyCode::Char(c @ '1'..='9') => {
            let idx = c.to_digit(10).unwrap() as usize - 1;
            if idx < app.action_items.len() {
                let action_id = app.action_items[idx].id.clone();
                app.close_action_palette();
                HandleResult::ExecuteAction(action_id)
            } else {
                HandleResult::Continue
            }
        }
        _ => HandleResult::Continue,
    }
}

/// Handle keys in search mode (inline filtering)
fn handle_search_mode(app: &mut App, key: KeyEvent) -> HandleResult {
    match key.code {
        KeyCode::Esc => {
            // Cancel filter
            app.cancel_filter();
            HandleResult::Continue
        }
        KeyCode::Enter => {
            // Apply filter and return to normal
            app.apply_filter();
            HandleResult::Continue
        }
        KeyCode::Backspace => {
            app.search_backspace();
            app.update_search_filter();
            HandleResult::UpdateFilter
        }
        KeyCode::Char(c) => {
            app.search_insert(c);
            app.update_search_filter();
            HandleResult::UpdateFilter
        }
        // Navigate results while filtering
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.select_next();
            HandleResult::Continue
        }
        KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.select_prev();
            HandleResult::Continue
        }
        KeyCode::Left => {
            if app.search_cursor > 0 {
                app.search_cursor -= 1;
            }
            HandleResult::Continue
        }
        KeyCode::Right => {
            if app.search_cursor < app.search_input.len() {
                app.search_cursor += 1;
            }
            HandleResult::Continue
        }
        _ => HandleResult::Continue,
    }
}

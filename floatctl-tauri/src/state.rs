//! State management for navigation and UI mode
//!
//! Handles the state machine for vim-like modal editing and hierarchical cursor.

use crate::protocol::{Cursor, Mode, NavigationState};

/// Keyboard input result
pub enum KeyResult {
    /// Key handled, state updated
    Handled,
    /// Key should propagate to edit mode
    Propagate,
    /// Key triggered a command
    Command(String),
}

/// Process a key press in normal mode
pub fn handle_normal_key(key: &str, state: &mut NavigationState) -> KeyResult {
    match key {
        // Vertical navigation
        "j" | "ArrowDown" => {
            state.cursor.index = state.cursor.index.saturating_add(1);
            KeyResult::Handled
        }
        "k" | "ArrowUp" => {
            state.cursor.index = state.cursor.index.saturating_sub(1);
            KeyResult::Handled
        }

        // Horizontal navigation (hierarchy)
        "l" | "ArrowRight" | "Enter" => {
            // Expand/enter item
            if let Some(ref item_id) = state.cursor.item_id {
                state.cursor.path.push(item_id.clone());
                state.cursor.depth += 1;
                state.cursor.index = 0;
            }
            KeyResult::Handled
        }
        "h" | "ArrowLeft" | "Backspace" => {
            // Go up in hierarchy
            if state.cursor.depth > 0 {
                state.cursor.path.pop();
                state.cursor.depth -= 1;
                state.cursor.index = 0;
            }
            KeyResult::Handled
        }

        // Jump to start/end
        "g" => KeyResult::Handled, // Wait for second key
        "G" => {
            state.cursor.index = usize::MAX; // Will be clamped by renderer
            KeyResult::Handled
        }

        // Mode switching
        "i" => {
            state.mode = Mode::Edit;
            KeyResult::Handled
        }
        ":" | "/" => {
            state.mode = Mode::Command;
            KeyResult::Command(key.to_string())
        }
        "v" => {
            state.mode = Mode::Visual;
            KeyResult::Handled
        }
        "Escape" => {
            state.mode = Mode::Normal;
            KeyResult::Handled
        }

        // Action shortcuts
        "Space" => KeyResult::Command("preview".to_string()),
        "e" => KeyResult::Command("edit".to_string()),
        "d" => KeyResult::Command("dispatch".to_string()),
        "x" => KeyResult::Command("delete".to_string()),

        _ => KeyResult::Propagate,
    }
}

/// Process a key press in command mode
pub fn handle_command_key(key: &str, state: &mut NavigationState) -> KeyResult {
    match key {
        "Escape" => {
            state.mode = Mode::Normal;
            KeyResult::Handled
        }
        "Enter" => {
            state.mode = Mode::Normal;
            KeyResult::Command("execute".to_string())
        }
        _ => KeyResult::Propagate,
    }
}

/// Clamp cursor index to valid range
pub fn clamp_cursor(cursor: &mut Cursor, max_index: usize) {
    if max_index == 0 {
        cursor.index = 0;
        cursor.item_id = None;
    } else {
        cursor.index = cursor.index.min(max_index - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_keys() {
        let mut state = NavigationState::default();
        state.cursor.index = 5;

        handle_normal_key("j", &mut state);
        assert_eq!(state.cursor.index, 6);

        handle_normal_key("k", &mut state);
        assert_eq!(state.cursor.index, 5);

        // Can't go below 0
        state.cursor.index = 0;
        handle_normal_key("k", &mut state);
        assert_eq!(state.cursor.index, 0);
    }

    #[test]
    fn mode_switching() {
        let mut state = NavigationState::default();
        assert_eq!(state.mode, Mode::Normal);

        handle_normal_key("i", &mut state);
        assert_eq!(state.mode, Mode::Edit);

        handle_normal_key("Escape", &mut state);
        assert_eq!(state.mode, Mode::Normal);
    }

    #[test]
    fn hierarchy_navigation() {
        let mut state = NavigationState::default();
        state.cursor.item_id = Some("board-1".into());

        handle_normal_key("l", &mut state);
        assert_eq!(state.cursor.depth, 1);
        assert_eq!(state.cursor.path, vec!["board-1"]);

        handle_normal_key("h", &mut state);
        assert_eq!(state.cursor.depth, 0);
        assert!(state.cursor.path.is_empty());
    }
}

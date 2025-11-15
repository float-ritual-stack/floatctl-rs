use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::mode::AppMode;

/// Render the command bar (bottom bar)
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let content = match app.mode {
        AppMode::Command => {
            // Show command input
            Line::from(vec![
                Span::styled(":", Style::default().fg(Color::Yellow)),
                Span::raw(&app.command_input),
                Span::styled("_", Style::default().fg(Color::Green)), // Cursor
            ])
        }

        AppMode::Normal | AppMode::Insert | AppMode::BoardNav => {
            // Show status message or keybind hints
            if let Some(ref msg) = app.status_message {
                Line::from(msg.as_str())
            } else {
                // Keybind hints based on mode
                let hints = match app.mode {
                    AppMode::Normal => {
                        "i: insert | :: command | b: boards | Tab: switch pane | q: quit"
                    }
                    AppMode::Insert => "Esc: normal | Ctrl-s: save",
                    AppMode::BoardNav => {
                        "w: work | t: tech | l: life-admin | r: recent | s: scratch | Esc: cancel"
                    }
                    _ => "",
                };

                Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray)))
            }
        }
    };

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, area);
}

use chrono::Local;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

/// Render the status bar (top bar)
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let mode_color = app.mode.color();
    let mode_name = app.mode.display_name();

    // Current time
    let now = Local::now();
    let time_str = now.format("%H:%M:%S").to_string();

    // Build status line
    let mut spans = vec![
        // Mode indicator
        Span::styled(
            format!(" {} ", mode_name),
            Style::default()
                .fg(Color::Black)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        // Current board
        Span::styled(
            format!("{}", app.current_board),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" "),
    ];

    // Add focused pane indicator
    let pane_indicator = match app.focused_pane {
        crate::mode::Pane::Scratch => "[SCRATCH]",
        crate::mode::Pane::Board => "[BOARD]",
    };
    spans.push(Span::styled(
        pane_indicator,
        Style::default().fg(Color::DarkGray),
    ));

    // Right-aligned time
    let width = area.width as usize;
    let current_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let padding = width.saturating_sub(current_len + time_str.len() + 2);

    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(
        format!("{} ", time_str),
        Style::default().fg(Color::DarkGray),
    ));

    let status_line = Line::from(spans);

    let paragraph = Paragraph::new(status_line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(paragraph, area);
}

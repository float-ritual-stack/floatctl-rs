use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use floatctl_tui::{App, BlockStore, UI};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize database
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let db_path = home.join(".floatctl").join("tui.db");
    let store = BlockStore::new(&db_path).await?;

    // Create app state
    let mut app = App::new(store);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create UI
    let mut ui = UI::new();

    // Load initial board data (state is in App now)
    app.load_board_blocks().await?;

    // Track last board for refresh detection
    let mut last_board = app.current_board.clone();

    // Main event loop
    let res = run_event_loop(&mut terminal, &mut app, &mut ui, &mut last_board).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    ui: &mut UI<'_>,
    last_board: &mut floatctl_tui::BoardId,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|f| ui.render(f, app))?;

        // Check if board changed (need to refresh)
        if app.current_board != *last_board {
            app.load_board_blocks().await?;
            *last_board = app.current_board.clone();
        }

        // Poll for events with timeout
        if let Some(event) = App::poll_event(Duration::from_millis(100))? {
            match event {
                Event::Key(key) => {
                    // Let UI handle input first (for insert mode)
                    ui.handle_input(key, app);

                    // Then let app handle global keys
                    // Skip if it was just a character in insert mode
                    let should_handle = match key.code {
                        KeyCode::Char(_) if app.mode == floatctl_tui::AppMode::Insert => false,
                        KeyCode::Backspace if app.mode == floatctl_tui::AppMode::Insert => false,
                        KeyCode::Enter if app.mode == floatctl_tui::AppMode::Insert => false,
                        _ => true,
                    };

                    if should_handle {
                        app.handle_key_event(key).await?;
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resized, will re-render on next loop
                }
                _ => {}
            }
        }

        // Exit if requested
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

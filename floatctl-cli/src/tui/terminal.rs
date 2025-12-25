//! Terminal management and main run loop

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::app::{App, MainTab, NavEntry};
use super::commands::{self, Command};
use super::event::{handle_key, poll_event, HandleResult};
use super::sources::{boards::BoardsSource, home::HomeSource, Source};
use super::ui;

/// Initialize the terminal for TUI mode
fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("Failed to create terminal")?;
    Ok(terminal)
}

/// Restore the terminal to normal mode
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

/// Run the TUI application
pub fn run() -> Result<()> {
    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create app state
    let mut app = App::new();

    // Initialize sources
    let home_source = HomeSource::new();
    let boards_source = BoardsSource::new();

    // Load initial data
    load_tab_data(&mut app, &home_source, &boards_source);

    // Main event loop
    let result = run_loop(&mut terminal, &mut app, &home_source, &boards_source);

    // Restore terminal (even if loop failed)
    restore_terminal(&mut terminal)?;

    result
}

/// Main event loop
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    home_source: &HomeSource,
    boards_source: &BoardsSource,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Poll for events (with 100ms timeout for responsive UI)
        if let Some(event) = poll_event(Duration::from_millis(100))? {
            match event {
                Event::Key(key) => {
                    // Dismiss help on any key if showing
                    if app.help_text.is_some() {
                        app.dismiss_help();
                        continue;
                    }

                    match handle_key(app, key) {
                        HandleResult::Quit => break,
                        HandleResult::Continue => {}
                        HandleResult::ExecuteAction(action_id) => {
                            execute_action(app, &action_id);
                        }
                        HandleResult::NavigateInto => {
                            navigate_into(app, home_source, boards_source);
                        }
                        HandleResult::NavigateBack => {
                            navigate_back(app, home_source, boards_source);
                        }
                        HandleResult::Refresh => {
                            app.search_state.clear();
                            load_tab_data(app, home_source, boards_source);
                        }
                        HandleResult::ExecuteCommand(cmd) => {
                            execute_command(app, cmd, home_source, boards_source);
                        }
                        HandleResult::UpdateFilter => {
                            // Filter already updated in app, just continue
                        }
                        HandleResult::SwitchTab(tab) => {
                            app.switch_tab(tab);
                            app.search_state.clear();
                            load_tab_data(app, home_source, boards_source);
                        }
                        HandleResult::ShowHelp => {
                            app.show_help(commands::get_help_text());
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resized, will be handled on next draw
                }
                _ => {}
            }
        }

        // Update preview for current selection
        update_preview(app, home_source, boards_source);

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Load data for the current tab
fn load_tab_data(app: &mut App, home_source: &HomeSource, boards_source: &BoardsSource) {
    // If we have navigation context, load children
    if let Some(nav) = &app.current_nav {
        let items = match nav.kind.as_str() {
            "home" => home_source.children(&nav.id),
            "board" | "boards" => boards_source.children(&nav.id),
            _ => None,
        };
        if let Some(items) = items {
            app.list_items = items;
            return;
        }
    }

    // Otherwise load root items for tab
    app.list_items = match app.main_tab {
        MainTab::Home => home_source.list(),
        MainTab::Boards => boards_source.list(),
        MainTab::Search => Vec::new(), // Search results will be populated by search
        MainTab::Dashboard => vec![
            super::app::ListItem {
                id: "tasks".to_string(),
                title: "Tasks".to_string(),
                subtitle: Some("View pending tasks".to_string()),
                kind: super::app::ItemKind::Folder,
                has_children: true,
                meta: None,
            },
            super::app::ListItem {
                id: "jobs".to_string(),
                title: "Background Jobs".to_string(),
                subtitle: Some("View job queue".to_string()),
                kind: super::app::ItemKind::Folder,
                has_children: true,
                meta: None,
            },
            super::app::ListItem {
                id: "activity".to_string(),
                title: "Activity Log".to_string(),
                subtitle: Some("Recent activity".to_string()),
                kind: super::app::ItemKind::Folder,
                has_children: true,
                meta: None,
            },
        ],
    };

    // Reset selection
    app.selected_index = 0;
    app.scroll_offset = 0;
    app.current_nav = None;
    app.nav_stack.clear();
}

/// Navigate into the selected item
fn navigate_into(app: &mut App, home_source: &HomeSource, boards_source: &BoardsSource) {
    if let Some(item) = app.selected_item().cloned() {
        if !item.has_children {
            return;
        }

        // Get children from appropriate source
        let (source_kind, children) = match app.main_tab {
            MainTab::Home => ("home", home_source.children(&item.id)),
            MainTab::Boards => {
                // Determine if it's a board or post
                if boards_source.children(&item.id).is_some() {
                    ("board", boards_source.children(&item.id))
                } else {
                    return;
                }
            }
            _ => return,
        };

        if let Some(children) = children {
            // Create nav entry
            let entry = NavEntry {
                kind: source_kind.to_string(),
                title: item.title.clone(),
                id: item.id.clone(),
                selected_index: 0,
            };

            app.navigate_into(entry);
            app.list_items = children;

            // Populate actions for this context
            let actions = match app.main_tab {
                MainTab::Home => home_source.actions(&item.id),
                MainTab::Boards => boards_source.actions(&item.id),
                _ => vec![],
            };
            app.action_items = actions;
        }
    }
}

/// Navigate back to parent
fn navigate_back(app: &mut App, home_source: &HomeSource, boards_source: &BoardsSource) {
    if app.navigate_back() {
        // Reload data for the level we're returning to
        if let Some(nav) = &app.current_nav {
            let items = match nav.kind.as_str() {
                "home" => home_source.children(&nav.id),
                "board" | "boards" => boards_source.children(&nav.id),
                _ => None,
            };
            if let Some(items) = items {
                app.list_items = items;
                return;
            }
        }

        // If we're at root, reload tab data
        load_tab_data(app, home_source, boards_source);
    }
}

/// Update preview content for current selection
fn update_preview(app: &mut App, home_source: &HomeSource, boards_source: &BoardsSource) {
    if !app.show_preview {
        return;
    }

    if let Some(item) = app.selected_item() {
        let preview = match app.main_tab {
            MainTab::Home => home_source.preview(&item.id),
            MainTab::Boards => boards_source.preview(&item.id),
            _ => None,
        };
        app.preview_content = preview;
    } else {
        app.preview_content = None;
    }
}

/// Execute a command from scratch pane
fn execute_command(
    app: &mut App,
    cmd: Command,
    home_source: &HomeSource,
    boards_source: &BoardsSource,
) {
    match cmd {
        Command::Search { query } => {
            if query.is_empty() {
                // Start filter mode
                app.start_filter();
            } else {
                // Search across all sources
                app.switch_tab(MainTab::Search);
                let mut results = Vec::new();
                results.extend(home_source.search(&query));
                results.extend(boards_source.search(&query));
                app.list_items = results;
                app.set_status(format!("Search: {} ({} results)", query, app.list_items.len()));
            }
        }
        Command::Board { name } => {
            // Find and navigate to board
            let boards = boards_source.list();
            if let Some(board) = boards.iter().find(|b| b.title.to_lowercase().contains(&name.to_lowercase())) {
                app.switch_tab(MainTab::Boards);
                let entry = NavEntry {
                    kind: "board".to_string(),
                    title: board.title.clone(),
                    id: board.id.clone(),
                    selected_index: 0,
                };
                if let Some(children) = boards_source.children(&board.id) {
                    app.navigate_into(entry);
                    app.list_items = children;
                    app.set_status(format!("Navigated to board: {}", board.title));
                }
            } else {
                app.set_status(format!("Board not found: {}", name));
            }
        }
        Command::Rag { query } => {
            if query.is_empty() {
                app.set_status("RAG search requires a query");
            } else {
                // Would integrate with floatctl_search here
                app.set_status(format!("RAG search: {} (not implemented)", query));
            }
        }
        Command::Open { path } => {
            if path.is_empty() {
                app.set_status("Open requires a path");
            } else {
                // Would open file/folder
                app.set_status(format!("Would open: {}", path));
            }
        }
        Command::Tab(tab) => {
            app.switch_tab(tab);
            app.search_state.clear();
            load_tab_data(app, home_source, boards_source);
            app.set_status(format!("Switched to {:?} tab", tab));
        }
        Command::Help => {
            app.show_help(commands::get_help_text());
        }
        Command::Clear => {
            app.clear_scratch();
            app.clear_status();
        }
        Command::Quit => {
            app.should_quit = true;
        }
        Command::Unknown { cmd, args } => {
            app.set_status(format!("Unknown command: /{} {}", cmd, args));
        }
    }
}

/// Execute an action
fn execute_action(app: &mut App, action_id: &str) {
    match action_id {
        "view" => {
            app.set_status("Viewing item...");
        }
        "open_editor" => {
            app.set_status("Would open in $EDITOR");
        }
        "edit_metadata" => {
            app.set_status("Metadata editing not implemented");
        }
        "copy" | "copy_path" => {
            if let Some(item) = app.selected_item() {
                // Would use cli-clipboard here
                app.set_status(format!("Copied: {}", item.id));
            }
        }
        "refactor_note" => {
            app.set_status("Bridge tender: refactor not implemented");
        }
        "new_post" => {
            app.set_status("New post not implemented");
        }
        "refresh" => {
            app.set_status("Refreshing...");
        }
        "delete" => {
            app.set_status("Delete not implemented (safety first!)");
        }
        _ => {
            app.set_status(format!("Unknown action: {}", action_id));
        }
    }
}

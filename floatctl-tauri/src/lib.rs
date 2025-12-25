//! Float Control - Tauri GUI Application
//!
//! A keyboard-first, terminal-inspired control center that eschews traditional
//! file explorer patterns in favor of hierarchical navigation (Boards → Posts → Actions).

pub mod protocol;
pub mod sources;
pub mod commands;
pub mod state;

pub use protocol::*;

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state shared across all windows
pub struct AppState {
    pub navigation: RwLock<NavigationState>,
    pub jobs: RwLock<Vec<JobProgress>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            navigation: RwLock::new(NavigationState::default()),
            jobs: RwLock::new(Vec::new()),
        }
    }
}

/// Initialize the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize shared state
            let state = Arc::new(AppState::default());
            app.manage(state);

            // Log startup
            tracing::info!("Float Control starting");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_items,
            commands::execute_action,
            commands::get_navigation_state,
            commands::set_mode,
            commands::navigate_to,
            commands::parse_scratch_command,
            commands::search,
            commands::get_actions_for_item,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

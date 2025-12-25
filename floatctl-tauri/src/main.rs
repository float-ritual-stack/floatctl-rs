//! Float Control GUI entry point

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    // Initialize tracing for development
    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt()
            .with_env_filter("floatctl_tauri=debug,tauri=info")
            .init();
    }

    floatctl_tauri::run();
}

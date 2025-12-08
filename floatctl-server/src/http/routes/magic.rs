//! Static file serving for /the-magic/
//!
//! Serves binaries and other static files from BBS_ROOT/the-magic/
//! This allows Desktop Claude to curl fresh binaries without skill rebundling.
//!
//! Example:
//! ```bash
//! curl -L -o /tmp/floatctl "https://float-bbs.ngrok.io/the-magic/floatctl-linux-x86_64"
//! chmod +x /tmp/floatctl
//! ```

use std::sync::Arc;

use axum::Router;
use tower_http::services::ServeDir;

use crate::http::server::AppState;

/// Create router for static file serving from /the-magic/
pub fn router() -> Router<Arc<AppState>> {
    // Serve files from BBS_ROOT/the-magic/
    // BBS_ROOT defaults to /opt/float/bbs, so files go in /opt/float/bbs/the-magic/
    let magic_dir = std::env::var("BBS_ROOT")
        .unwrap_or_else(|_| "/opt/float/bbs".to_string());
    let magic_path = format!("{}/the-magic", magic_dir);

    tracing::info!(path = %magic_path, "Serving static files from /the-magic/");

    Router::new().nest_service("/the-magic", ServeDir::new(magic_path))
}

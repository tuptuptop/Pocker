//! Pocker agent backend — the Rust-native replacement for the Python Hermes
//! agent sidecar.
//!
//! Exposes an HTTP server (default port 8642) implementing the Hermes
//! FastAPI contract that PockerStudio expects, backed by OpenAI-compatible
//! LLM providers. Run standalone via `pocker-agent`, or embedded through
//! `pocker agent serve` in the main CLI.

pub mod config;
pub mod provider;
pub mod server;
pub mod state;

pub use config::{Config, Provider};
pub use state::{AppState, Session};

use std::sync::Arc;

/// Bind and serve the agent HTTP API.
///
/// Tracing is intentionally *not* initialised here so callers (the
/// `pocker-agent` binary or the parent `pocker` CLI) own log setup.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new(config.clone()));
    let app = server::build_router(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(
        "pocker-agent listening on http://{addr} (provider={})",
        config.provider.as_str()
    );
    axum::serve(listener, app).await?;
    Ok(())
}

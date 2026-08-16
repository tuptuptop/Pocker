//! Pocker Hub — plugin registry and distribution platform server.
//!
//! Provides REST API for:
//! - Plugin search/discovery
//! - Plugin publish/pull
//! - Version management
//! - Rating system
//! - Security scanning

mod api;
mod store;

use std::net::SocketAddr;
use tracing::info;

/// Run the Hub server.
///
/// # Errors
/// Returns an error if the TCP listener cannot bind to `addr` or the server
/// terminates with an error.
pub async fn run(addr: SocketAddr) -> anyhow::Result<()> {
    let app = api::build_router();

    info!("Pocker Hub server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pocker_hub=info".to_string())
        .init();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    run(addr).await
}

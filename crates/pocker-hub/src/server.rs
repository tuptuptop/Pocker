//! Pocker Hub server.

use crate::api;
use pocker_engine::Engine;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// Run the Hub server.
///
/// # Errors
/// Returns an error if the TCP listener cannot bind to `addr` or the server
/// terminates with an error.
pub async fn run(addr: SocketAddr, engine: Arc<Engine>) -> anyhow::Result<()> {
    let app = api::build_router(engine);

    info!("Pocker Hub server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

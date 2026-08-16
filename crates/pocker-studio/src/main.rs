//! Pocker Studio — Web GUI backend.
//!
//! Serves the TypeScript frontend (based on Hermes-Studio) and provides
//! the REST API + WebSocket for real-time communication.
//!
//! Architecture:
//! - Rust backend: API server, SSE streaming, WebSocket
//! - TypeScript frontend: React + `TanStack` + Zustand (in studio-web/)

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

mod api;

/// Run the Studio server.
///
/// # Errors
/// Returns an error if the underlying HTTP server fails to bind or terminates
/// with an error.
pub async fn run(addr: SocketAddr, static_dir: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let mut router = Router::new()
        .merge(api::build_router())
        .layer(CorsLayer::permissive());

    // Serve static files if the frontend has been built
    if let Some(dir) = static_dir {
        if dir.exists() {
            info!("Serving static frontend from: {:?}", dir);
            router = router.fallback_service(ServeDir::new(dir));
        }
    }

    info!("Pocker Studio starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pocker_studio=info".to_string())
        .init();

    let addr: SocketAddr = "127.0.0.1:3080".parse()?;

    // Look for built frontend in conventional locations
    let static_dir = dirs::home_dir().map(|h| h.join(".pocker").join("studio").join("dist"));

    run(addr, static_dir).await
}

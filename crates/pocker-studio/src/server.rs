//! Pocker Studio server.

use crate::api;
use axum::Router;
use pocker_engine::Engine;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

/// Run the Studio server.
///
/// # Errors
/// Returns an error if the underlying HTTP server fails to bind or terminates
/// with an error.
pub async fn run(
    addr: SocketAddr,
    static_dir: Option<std::path::PathBuf>,
    engine: Arc<Engine>,
) -> anyhow::Result<()> {
    let mut router = Router::new()
        .merge(api::build_router(engine))
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

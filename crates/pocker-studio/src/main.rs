//! Pocker Studio — binary entry point.

use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pocker_studio=info".to_string())
        .init();

    let addr: SocketAddr = "127.0.0.1:3080".parse()?;

    // Look for built frontend in conventional locations
    let static_dir = dirs::home_dir().map(|h| h.join(".pocker").join("studio").join("dist"));

    pocker_studio::run(addr, static_dir, Arc::new(pocker_engine::Engine::new())).await
}

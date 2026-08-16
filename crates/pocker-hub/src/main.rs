//! Pocker Hub — binary entry point.

use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pocker_hub=info".to_string())
        .init();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    pocker_hub::run(addr, Arc::new(pocker_engine::Engine::new())).await
}

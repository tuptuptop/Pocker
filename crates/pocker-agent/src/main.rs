//! `pocker-agent` standalone binary — Rust-native agent backend.

use pocker_agent::{serve, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pocker_agent=info".into()),
        )
        .init();

    let config = Config::from_env();
    tracing::info!(
        "starting pocker-agent (provider={}, model={})",
        config.provider.as_str(),
        config.model
    );
    serve(config).await
}

# Rust-native Pocker agent backend.
#
# Replaces the Python Hermes agent sidecar entirely. Builds the `pocker-agent`
# binary from the workspace and serves the PockerStudio-compatible HTTP API on
# port 8642 (Hermes contract: /health, /v1/chat/completions, /api/sessions/*).

FROM rust:1.82-slim AS builder
WORKDIR /app
# Build context is the workspace root (see docker-compose.yml `context: ..`),
# so the whole Rust workspace (crates/pocker-*) is available here.
COPY . .
RUN cargo build --release -p pocker-agent

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/pocker-agent /usr/local/bin/pocker-agent

EXPOSE 8642

HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=15s \
    CMD curl -f http://localhost:8642/health || exit 1

ENTRYPOINT ["pocker-agent"]
CMD ["serve", "--port", "8642"]

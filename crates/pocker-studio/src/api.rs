//! Studio API routes.

use axum::{routing::{get, post}, Router, Json, extract::State};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct StudioState {
    // TODO: Add engine reference
}

pub fn build_router() -> Router {
    let state = StudioState {};

    Router::new()
        .route("/api/health", get(health))
        .route("/api/version", get(version))
        .route("/api/plugins", get(list_plugins))
        .route("/api/profiles", get(list_profiles))
        .route("/api/chat", post(chat))
        .with_state(state)
}

async fn health() -> Json<Value> {
    json!({
        "status": "ok",
        "service": "pocker-studio"
    }).into()
}

async fn version() -> Json<Value> {
    json!({
        "name": "pocker-studio",
        "version": env!("CARGO_PKG_VERSION"),
    }).into()
}

async fn list_plugins(State(_state): State<StudioState>) -> Json<Value> {
    // TODO: Query from engine
    json!({
        "plugins": [],
    }).into()
}

async fn list_profiles(State(_state): State<StudioState>) -> Json<Value> {
    // TODO: Query from profile manager
    json!({
        "profiles": ["web", "cli", "tui", "headless"],
        "current": "web",
    }).into()
}

#[derive(serde::Deserialize)]
struct ChatRequest {
    message: String,
}

async fn chat(
    State(_state): State<StudioState>,
    Json(req): Json<ChatRequest>,
) -> Json<Value> {
    // TODO: Forward to engine → agent loop → LLM
    json!({
        "reply": format!("Echo: {} (engine not yet connected)", req.message),
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0,
        },
    }).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health() {
        let app = build_router();
        let response = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let app = build_router();
        let response = app
            .oneshot(Request::builder().uri("/api/profiles").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

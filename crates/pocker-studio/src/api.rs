//! Studio API routes.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use pocker_engine::Engine;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct StudioState {
    /// Shared engine; the source of truth for live plugin metadata + status.
    engine: Arc<Engine>,
}

pub fn build_router(engine: Arc<Engine>) -> Router {
    let state = StudioState { engine };

    Router::new()
        .route("/api/health", get(health))
        .route("/api/version", get(version))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/:name", get(plugin_info))
        .route("/api/profiles", get(list_profiles))
        .route("/api/chat", post(chat))
        .with_state(state)
}

async fn health() -> Json<Value> {
    json!({
        "status": "ok",
        "service": "pocker-studio"
    })
    .into()
}

async fn version() -> Json<Value> {
    json!({
        "name": "pocker-studio",
        "version": env!("CARGO_PKG_VERSION"),
    })
    .into()
}

async fn list_plugins(State(state): State<StudioState>) -> Json<Value> {
    let loader = &state.engine.loader;
    let plugins: Vec<Value> = loader
        .list()
        .into_iter()
        .map(|(name, mounted)| {
            json!({
                "name": name,
                "mounted": mounted,
                "metadata": loader.metadata(&name),
            })
        })
        .collect();
    json!({
        "plugins": plugins,
        "total": plugins.len(),
    })
    .into()
}

async fn plugin_info(
    State(state): State<StudioState>,
    Path(name): Path<String>,
) -> Json<Value> {
    let loader = &state.engine.loader;
    match loader.metadata(&name) {
        Some(meta) => json!({
            "name": name,
            "found": true,
            "mounted": loader.is_mounted(&name),
            "metadata": meta,
        })
        .into(),
        None => json!({
            "name": name,
            "found": false,
            "message": "Plugin not found",
        })
        .into(),
    }
}

async fn list_profiles(State(state): State<StudioState>) -> Json<Value> {
    let profiles = state.engine.list_profiles().unwrap_or_default();
    let current = state.engine.current_profile();
    json!({
        "profiles": profiles,
        "current": current,
    })
    .into()
}

#[derive(serde::Deserialize)]
struct ChatRequest {
    message: String,
}

async fn chat(State(_state): State<StudioState>, Json(req): Json<ChatRequest>) -> Json<Value> {
    // TODO: Forward to engine → agent loop → LLM
    json!({
        "reply": format!("Echo: {} (engine not yet connected)", req.message),
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0,
        },
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn router() -> Router {
        build_router(Arc::new(Engine::new()))
    }

    #[tokio::test]
    async fn test_health() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/plugins")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert!(value["plugins"].is_array());
        assert!(value["total"].is_number());
    }

    #[tokio::test]
    async fn test_plugin_info_found() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/plugins/@pocker/core")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        // `@pocker/core` is registered as a factory but not mounted until a
        // profile loads it, so found=true but mounted=false is expected here.
        assert_eq!(value["found"], true);
        assert_eq!(value["metadata"]["name"], "@pocker/core");
    }

    #[tokio::test]
    async fn test_plugin_info_not_found() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/plugins/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["found"], false);
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/profiles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert!(value["profiles"].is_array());
    }
}

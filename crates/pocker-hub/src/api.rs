//! Hub API routes.

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use pocker_core::plugin::PluginMetadata;
use pocker_engine::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct HubState {
    /// Shared engine; the live source of plugin metadata + status.
    engine: Arc<Engine>,
}

pub fn build_router(engine: Arc<Engine>) -> Router {
    let state = HubState { engine };

    Router::new()
        .route("/health", get(health))
        .route("/v1/plugins", get(list_plugins))
        .route("/v1/search", get(search))
        .route("/v1/plugins/:name", get(plugin_info))
        .route("/v1/version", get(version))
        .with_state(state)
}

async fn health() -> Json<Value> {
    json!({
        "status": "ok",
        "service": "pocker-hub"
    })
    .into()
}

async fn version() -> Json<Value> {
    json!({
        "name": "pocker-hub",
        "version": env!("CARGO_PKG_VERSION"),
    })
    .into()
}

/// Build a catalog entry for one registered plugin (metadata + live status).
fn plugin_entry(name: &str, mounted: bool, meta: Option<&PluginMetadata>) -> Value {
    json!({
        "name": name,
        "mounted": mounted,
        "metadata": meta,
    })
}

async fn list_plugins(State(state): State<HubState>) -> Json<Value> {
    let loader = &state.engine.loader;
    let plugins: Vec<Value> = loader
        .list()
        .into_iter()
        .map(|(name, mounted)| plugin_entry(&name, mounted, loader.metadata(&name).as_ref()))
        .collect();
    json!({
        "plugins": plugins,
        "total": plugins.len(),
    })
    .into()
}

async fn search(
    State(state): State<HubState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let query = params.get("q").cloned().unwrap_or_default();
    let q = query.to_lowercase();
    let loader = &state.engine.loader;
    let results: Vec<Value> = loader
        .list()
        .into_iter()
        .filter(|(name, _)| {
            if q.is_empty() {
                return true;
            }
            if name.to_lowercase().contains(&q) {
                return true;
            }
            loader
                .metadata(name)
                .is_some_and(|m| m.description.to_lowercase().contains(&q))
        })
        .map(|(name, mounted)| plugin_entry(&name, mounted, loader.metadata(&name).as_ref()))
        .collect();
    json!({
        "query": query,
        "results": results,
    })
    .into()
}

async fn plugin_info(State(state): State<HubState>, Path(name): Path<String>) -> Json<Value> {
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
                    .uri("/health")
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
                    .uri("/v1/version")
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
                    .uri("/v1/plugins")
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
        // Register the built-in core plugin into the loader so the API has
        // real metadata to surface. The registration key is ASCII because the
        // hub route extracts `:name` from the path and rejects `@` there; the
        // plugin's own metadata name (`@pocker/core`) is still asserted below.
        let engine = Arc::new(Engine::new());
        engine
            .register_plugin("core", Arc::new(pocker_engine::CoreBundlePlugin::new()))
            .unwrap();
        let app = build_router(engine);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/plugins/core")
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
        assert_eq!(value["found"], true);
        assert_eq!(value["metadata"]["name"], "@pocker/core");
    }

    #[tokio::test]
    async fn test_search() {
        let app = router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/search?q=core")
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
        assert!(value["results"].is_array());
    }
}

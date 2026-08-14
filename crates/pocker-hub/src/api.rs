//! Hub API routes.

use axum::{routing::get, Router, Json};
use serde_json::{json, Value};

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/plugins", get(list_plugins))
        .route("/v1/search", get(search))
        .route("/v1/plugins/:name", get(plugin_info))
        .route("/v1/version", get(version))
}

async fn health() -> Json<Value> {
    json!({
        "status": "ok",
        "service": "pocker-hub"
    }).into()
}

async fn version() -> Json<Value> {
    json!({
        "name": "pocker-hub",
        "version": env!("CARGO_PKG_VERSION"),
    }).into()
}

async fn list_plugins() -> Json<Value> {
    // TODO: Query from database
    json!({
        "plugins": [],
        "total": 0,
    }).into()
}

async fn search(axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let query = params.get("q").cloned().unwrap_or_default();
    // TODO: Search in database
    json!({
        "query": query,
        "results": [],
    }).into()
}

async fn plugin_info(axum::extract::Path(name): axum::extract::Path<String>) -> Json<Value> {
    // TODO: Fetch from database
    json!({
        "name": name,
        "found": false,
        "message": "Plugin not found",
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
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version() {
        let app = build_router();
        let response = app
            .oneshot(Request::builder().uri("/v1/version").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let app = build_router();
        let response = app
            .oneshot(Request::builder().uri("/v1/plugins").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_search() {
        let app = build_router();
        let response = app
            .oneshot(Request::builder().uri("/v1/search?q=test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

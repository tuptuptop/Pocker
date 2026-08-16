//! Axum HTTP server implementing the Hermes FastAPI contract that
//! PockerStudio depends on, now backed by the Rust agent runtime instead of
//! the Python sidecar.

use crate::provider;
use crate::state::{AppState, Session};
use axum::extract::{Path, State};
use axum::response::sse::Sse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

/// Build the router for the agent backend.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/search", get(search_sessions))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id/chat", post(chat_nonstream))
        .route("/api/sessions/:id/chat/stream", post(chat_stream_hermes))
        .route("/api/sessions/:id/messages", get(messages))
        .route("/api/sessions/:id/fork", post(fork_session))
        .route("/api/memory", get(memory))
        .route("/api/skills", get(skills))
        .route("/api/skills/categories", get(skills_categories))
        .route("/api/config", get(config_route))
        .with_state(state)
}

fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn health() -> impl axum::response::IntoResponse {
    Json(json!({ "status": "ok" }))
}

fn models(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    let m = &state.config.model;
    Json(json!({
        "object": "list",
        "data": [{ "id": m, "object": "model", "owned_by": "pocker-agent" }]
    }))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    provider::proxy_chat_completions(&state.config, body).await
}

fn create_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let title = body.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
    let model = body.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
    let started = now();
    let sess = Session {
        id: id.clone(),
        title: title.clone(),
        model: model.clone(),
        started_at: started,
    };
    state.sessions.write().unwrap().insert(id.clone(), sess);
    Json(json!({
        "session": {
            "id": id,
            "title": title,
            "model": model,
            "started_at": started,
            "ended_at": null,
        }
    }))
}

fn list_sessions(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    let map = state.sessions.read().unwrap();
    let items: Vec<Value> = map
        .values()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "model": s.model,
                "started_at": s.started_at,
                "ended_at": null,
            })
        })
        .collect();
    Json(json!({ "items": items, "total": items.len() }))
}

fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    let map = state.sessions.read().unwrap();
    match map.get(&id) {
        Some(s) => Json(json!({
            "session": {
                "id": s.id,
                "title": s.title,
                "model": s.model,
                "started_at": s.started_at,
                "ended_at": null,
            }
        }))
        .into_response(),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({ "error": "session not found" })),
        )
            .into_response(),
    }
}

fn search_sessions(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let q = params.get("q").cloned().unwrap_or_default();
    Json(json!({ "query": q, "count": 0, "results": [] }))
}

async fn chat_nonstream(
    State(state): State<Arc<AppState>>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let message = body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = body.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
    let content = provider::complete_chat(&state.config, message, model).await;
    Json(json!({ "content": content }))
}

async fn chat_stream_hermes(
    State(state): State<Arc<AppState>>,
    Path(_id): Path<String>,
    Json(body): Json<Value>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let message = body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = body.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
    provider::hermes_chat_stream(state.config.clone(), message, model).await
}

fn messages(
    _state: State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> impl axum::response::IntoResponse {
    Json(json!({ "items": [], "total": 0 }))
}

fn fork_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    let new_id = uuid::Uuid::new_v4().to_string();
    let (title, model, started) = {
        let map = state.sessions.read().unwrap();
        let s = map.get(&id);
        (
            s.map(|s| s.title.clone()),
            s.map(|s| s.model.clone()),
            now(),
        )
    };
    let sess = Session {
        id: new_id.clone(),
        title,
        model,
        started_at: started,
    };
    state.sessions.write().unwrap().insert(new_id.clone(), sess);
    Json(json!({
        "session": {
            "id": new_id,
            "title": null,
            "model": null,
            "started_at": started,
            "ended_at": null,
        },
        "forked_from": id,
    }))
}

fn memory() -> impl axum::response::IntoResponse {
    Json(json!({ "object": "memory", "data": [] }))
}

fn skills() -> impl axum::response::IntoResponse {
    Json(json!([]))
}

fn skills_categories() -> impl axum::response::IntoResponse {
    Json(json!([]))
}

fn config_route(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    Json(json!({
        "model": state.config.model,
        "provider": state.config.provider.as_str(),
        "base_url": state.config.base_url,
    }))
}

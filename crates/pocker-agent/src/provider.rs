//! LLM provider integration for the Pocker agent backend.
//!
//! This module speaks directly to OpenAI-compatible chat-completions
//! streaming endpoints (OpenAI and OpenRouter both use this shape). The
//! `/v1/chat/completions` route proxies the upstream SSE verbatim, while
//! `/api/sessions/:id/chat/stream` re-wraps the upstream OpenAI SSE into the
//! Hermes `assistant.delta` / `assistant.completed` event format that
//! PockerStudio expects.
//!
//! (Anthropic-native streaming and an embedded `rig`-based agent loop are
//! planned follow-ups; this layer keeps the transport explicit and testable.)

use crate::config::Config;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use futures::stream::Stream;
use futures::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Proxy a client chat-completions request upstream and stream the raw
/// OpenAI-compatible SSE back to the caller. The caller (PockerStudio's
/// `openai-compat` backend) understands this format natively.
pub async fn proxy_chat_completions(cfg: &Config, mut body: Value) -> Response {
    let upstream = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));

    // Always provide a model + force streaming.
    let model_missing = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.is_empty())
        .unwrap_or(true);
    if model_missing {
        body["model"] = json!(cfg.model);
    }
    body["stream"] = json!(true);

    let client = reqwest::Client::new();
    let built = client
        .post(&upstream)
        .bearer_auth(&cfg.api_key)
        .header("content-type", "application/json")
        .json(&body);

    match built.send().await {
        Ok(resp) if resp.status().is_success() => {
            let stream = resp.bytes_stream();
            Body::from_stream(stream)
                .into_response()
                .into_response_with_sse_headers()
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (StatusCode::BAD_GATEWAY, format!("upstream error {status}: {text}")).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

/// Stream a single user message through the upstream model and re-wrap the
/// OpenAI SSE into Hermes `assistant.delta` / `assistant.completed` events.
pub async fn hermes_chat_stream(
    cfg: Config,
    message: String,
    model: Option<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel::<Result<Event, Infallible>>();

    tokio::spawn(async move {
        let upstream = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
        let body = json!({
            "model": model.unwrap_or(cfg.model),
            "messages": [{"role": "user", "content": message}],
            "stream": true,
        });

        let client = reqwest::Client::new();
        match client
            .post(&upstream)
            .bearer_auth(&cfg.api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let mut stream = resp.bytes_stream().map(|r| r.unwrap_or_default());
                let mut buf = String::new();
                let mut full = String::new();

                while let Some(chunk) = StreamExt::next(&mut stream).await {
                    buf.push_str(&String::from_utf8_lossy(&chunk));
                    let mut start = 0;
                    while let Some(idx) = buf[start..].find("\n\n") {
                        let raw = buf[start..start + idx].to_string();
                        start += idx + 2;
                        for line in raw.split('\n') {
                            let line = line.trim();
                            if let Some(payload) = line.strip_prefix("data:") {
                                let payload = payload.trim();
                                if payload.is_empty() || payload == "[DONE]" {
                                    continue;
                                }
                                if let Ok(v) = serde_json::from_str::<Value>(payload) {
                                    if let Some(delta) =
                                        v["choices"][0]["delta"]["content"].as_str()
                                    {
                                        if !delta.is_empty() {
                                            full.push_str(delta);
                                            let _ = tx.send(Ok(
                                                Event::default()
                                                    .event("assistant.delta")
                                                    .data(delta),
                                            ));
                                        }
                                    }
                                    let reasoning = v["choices"][0]["delta"]
                                        ["reasoning_content"]
                                        .as_str()
                                        .or_else(|| {
                                            v["choices"][0]["delta"]["reasoning"].as_str()
                                        })
                                        .unwrap_or("");
                                    if !reasoning.is_empty() {
                                        let _ = tx.send(Ok(Event::default()
                                            .event("assistant.delta")
                                            .data(json!({"reasoning": reasoning}).to_string())));
                                    }
                                }
                            }
                        }
                    }
                    buf.drain(..start);
                }

                let _ = tx.send(Ok(Event::default()
                    .event("assistant.completed")
                    .data(json!({ "content": full }).to_string())));
            }
            Ok(resp) => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(format!("upstream error: {}", resp.status()))));
                let _ = tx.send(Ok(Event::default()
                    .event("assistant.completed")
                    .data(json!({ "content": "" }).to_string())));
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default().event("error").data(e.to_string())));
                let _ = tx.send(Ok(Event::default()
                    .event("assistant.completed")
                    .data(json!({ "content": "" }).to_string())));
            }
        }
    });

    Sse::new(UnboundedReceiverStream::new(rx))
}

/// Small helper to stamp the SSE content-type / cache headers on a proxied
/// response without forcing callers to build headers by hand.
trait SseResponseExt {
    fn into_response_with_sse_headers(self) -> Response;
}

impl SseResponseExt for Response {
    fn into_response_with_sse_headers(self) -> Response {
        let mut builder = Response::builder()
            .status(self.status())
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive");
        if let Some(hv) = self.headers().get("access-control-allow-origin") {
            builder = builder.header("access-control-allow-origin", hv);
        }
        match builder.body(self.into_body()) {
            Ok(r) => r,
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "build failed").into_response(),
        }
    }
}

/// One-shot (non-streaming) completion used by the `/api/sessions/:id/chat`
/// route. Returns the assistant text, or an empty string on any failure.
pub async fn complete_chat(cfg: &Config, message: String, model: Option<String>) -> String {
    let upstream = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let body = json!({
        "model": model.unwrap_or_else(|| cfg.model.clone()),
        "messages": [{"role": "user", "content": message}],
        "stream": false,
    });
    let client = reqwest::Client::new();
    match client
        .post(&upstream)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(v) => v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            Err(_) => String::new(),
        },
        _ => String::new(),
    }
}

//! Shared application state for the agent HTTP server.

use crate::config::Config;
use std::collections::HashMap;
use std::sync::RwLock;

/// An in-memory chat session (mirrors the Hermes `/api/sessions` shape that
/// PockerStudio expects). Persisted sessions are a future enhancement;
/// for the local sidecar use-case an in-memory store is sufficient.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub model: Option<String>,
    pub started_at: u64,
}

#[derive(Default)]
pub struct AppState {
    pub config: Config,
    pub sessions: RwLock<HashMap<String, Session>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

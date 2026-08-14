//! Shared Context (`Ctx`) — the heart of Pocker.
//!
//! The context holds the seam registry, event map, and configuration.
//! Plugins register their services into the context via seams.
//! There is no privileged core; the context is just a shared bus.

use crate::event::EventMap;
use crate::seam::{SeamRegistry, SeamId, Seam};
use crate::types::Profile;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

/// The shared context, accessible to all plugins.
///
/// This is analogous to DSH's Cordis context: plugins contribute
/// services, typed events, and reversible effects to this shared context.
pub struct Ctx {
    /// Registry of all seams (capability extension points)
    seams: RwLock<SeamRegistry>,
    /// Event map (typed events + handlers)
    events: RwLock<EventMap>,
    /// Configuration values (merged from profiles + patches)
    config: RwLock<serde_json::Value>,
    /// The active profile
    profile: RwLock<Profile>,
    /// Arbitrary key-value store for plugin-specific data
    store: RwLock<HashMap<String, serde_json::Value>>,
}

impl Ctx {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            seams: RwLock::new(SeamRegistry::new()),
            events: RwLock::new(EventMap::new()),
            config: RwLock::new(serde_json::Value::Null),
            profile: RwLock::new(Profile {
                name: "default".to_string(),
                description: "Default profile".to_string(),
                bundles: Vec::new(),
                plugins: Vec::new(),
                patch: serde_json::Value::Null,
            }),
            store: RwLock::new(HashMap::new()),
        }
    }

    /// Register an implementation on a seam.
    pub fn register_seam(
        &self,
        seam: SeamId,
        provider: String,
        implementation: Arc<dyn Seam>,
    ) {
        let mut seams = self.seams.write().unwrap();
        tracing::debug!(seam = %seam, provider = %provider, "registering seam");
        seams.register(seam, provider, implementation);
    }

    /// Unregister a provider's implementation from a seam.
    pub fn unregister_seam(&self, seam: &SeamId, provider: &str) {
        let mut seams = self.seams.write().unwrap();
        tracing::debug!(seam = %seam, provider = %provider, "unregistering seam");
        seams.unregister_provider(seam, provider);
    }

    /// Get the default implementation for a seam.
    pub fn get_seam(&self, seam: &SeamId) -> Option<Arc<dyn Seam>> {
        let seams = self.seams.read().unwrap();
        seams.get(seam)
    }

    /// Check if a seam is available.
    pub fn has_seam(&self, seam: &SeamId) -> bool {
        let seams = self.seams.read().unwrap();
        seams.has(seam)
    }

    /// List all registered seams.
    pub fn list_seams(&self) -> Vec<SeamId> {
        let seams = self.seams.read().unwrap();
        seams.list()
    }

    /// Subscribe to an event.
    pub fn on(&self, event_name: &str, handler: crate::event::EventHandler) {
        let mut events = self.events.write().unwrap();
        events.subscribe(event_name, handler);
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: &crate::event::Event) {
        let events = self.events.read().unwrap();
        events.emit(event);
    }

    /// Get a configuration value by path (e.g. "llm.default_model").
    pub fn config_get(&self, path: &str) -> Option<serde_json::Value> {
        let config = self.config.read().unwrap();
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &*config;
        for part in parts {
            if let serde_json::Value::Object(map) = current {
                if let Some(v) = map.get(part) {
                    current = v;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(current.clone())
    }

    /// Set the full configuration.
    pub fn set_config(&self, config: serde_json::Value) {
        let mut cfg = self.config.write().unwrap();
        *cfg = config;
    }

    /// Set the active profile.
    pub fn set_profile(&self, profile: Profile) {
        let mut p = self.profile.write().unwrap();
        *p = profile;
    }

    /// Get the active profile name.
    pub fn profile_name(&self) -> String {
        let p = self.profile.read().unwrap();
        p.name.clone()
    }

    /// Store an arbitrary value.
    pub fn store_set(&self, key: &str, value: serde_json::Value) {
        let mut store = self.store.write().unwrap();
        store.insert(key.to_string(), value);
    }

    /// Retrieve a stored value.
    pub fn store_get(&self, key: &str) -> Option<serde_json::Value> {
        let store = self.store.read().unwrap();
        store.get(key).cloned()
    }

    /// Dump the current state for debugging (like `dsh --dump-config`).
    pub fn dump(&self) -> serde_json::Value {
        let seams = self.seams.read().unwrap();
        let config = self.config.read().unwrap();
        let profile = self.profile.read().unwrap();

        serde_json::json!({
            "profile": profile.name,
            "seams": seams.list().iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "config": config.clone(),
        })
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummySeam {
        name: String,
    }

    impl Seam for DummySeam {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn ctx_register_and_get_seam() {
        let ctx = Ctx::new();
        let seam = SeamId::llm();
        let impl_arc = Arc::new(DummySeam {
            name: "openai".to_string(),
        }) as Arc<dyn Seam>;

        ctx.register_seam(seam.clone(), "openai-plugin".to_string(), impl_arc);

        assert!(ctx.has_seam(&seam));
        let got = ctx.get_seam(&seam).unwrap();
        assert_eq!(got.name(), "openai");
    }

    #[test]
    fn ctx_unregister_seam() {
        let ctx = Ctx::new();
        let seam = SeamId::tools();
        let impl_arc = Arc::new(DummySeam {
            name: "shell".to_string(),
        }) as Arc<dyn Seam>;

        ctx.register_seam(seam.clone(), "shell-plugin".to_string(), impl_arc);
        assert!(ctx.has_seam(&seam));

        ctx.unregister_seam(&seam, "shell-plugin");
        assert!(!ctx.has_seam(&seam));
    }

    #[test]
    fn ctx_config_get() {
        let ctx = Ctx::new();
        ctx.set_config(serde_json::json!({
            "llm": {
                "default_model": "deepseek-chat"
            }
        }));

        assert_eq!(
            ctx.config_get("llm.default_model"),
            Some(serde_json::json!("deepseek-chat"))
        );
        assert_eq!(ctx.config_get("llm.nonexistent"), None);
        assert_eq!(ctx.config_get("nonexistent"), None);
    }

    #[test]
    fn ctx_store() {
        let ctx = Ctx::new();
        ctx.store_set("key1", serde_json::json!("value1"));
        assert_eq!(ctx.store_get("key1"), Some(serde_json::json!("value1")));
        assert_eq!(ctx.store_get("key2"), None);
    }

    #[test]
    fn ctx_dump() {
        let ctx = Ctx::new();
        ctx.set_config(serde_json::json!({"test": true}));
        let dump = ctx.dump();
        assert_eq!(dump["profile"], "default");
        assert_eq!(dump["config"]["test"], true);
    }
}

//! Shared Context (`Ctx`) — the heart of Pocker.
//!
//! The context holds the seam registry, event map, and configuration.
//! Plugins register their services into the context via seams.
//! There is no privileged core; the context is just a shared bus.

use crate::event::EventMap;
use crate::seam::{Seam, SeamId, SeamRegistry};
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
    #[must_use]
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
    ///
    /// # Panics
    /// Panics if the internal seam registry lock is poisoned.
    pub fn register_seam(&self, seam: SeamId, provider: String, implementation: Arc<dyn Seam>) {
        let mut seams = self.seams.write().unwrap();
        tracing::debug!(seam = %seam, provider = %provider, "registering seam");
        seams.register(seam, provider, implementation);
    }

    /// Unregister a provider's implementation from a seam.
    ///
    /// # Panics
    /// Panics if the internal seam registry lock is poisoned.
    pub fn unregister_seam(&self, seam: &SeamId, provider: &str) {
        let mut seams = self.seams.write().unwrap();
        tracing::debug!(seam = %seam, provider = %provider, "unregistering seam");
        seams.unregister_provider(seam, provider);
    }

    /// Get the default implementation for a seam.
    ///
    /// # Panics
    /// Panics if the internal seam registry lock is poisoned.
    pub fn get_seam(&self, seam: &SeamId) -> Option<Arc<dyn Seam>> {
        let seams = self.seams.read().unwrap();
        seams.get(seam)
    }

    /// Get a seam as a concrete typed service.
    ///
    /// Downcasts the stored `Arc<dyn Seam>` to `Arc<T>`. Returns `None` if the
    /// seam is absent or stores a different concrete type. Requires `Seam: Any`
    /// (see [`crate::seam::Seam`]).
    pub fn get_seam_typed<T: Seam + 'static>(&self, seam: &SeamId) -> Option<Arc<T>> {
        let stored: Arc<dyn Seam> = self.get_seam(seam)?;
        // Coerce `Arc<dyn Seam>` (Seam: Any + Send + Sync + 'static) to the
        // Any trait object so we can downcast back to the concrete type.
        let as_any: Arc<dyn std::any::Any + Send + Sync> = stored;
        Arc::downcast::<T>(as_any).ok()
    }

    /// Check if a seam is available.
    ///
    /// # Panics
    /// Panics if the internal seam registry lock is poisoned.
    pub fn has_seam(&self, seam: &SeamId) -> bool {
        let seams = self.seams.read().unwrap();
        seams.has(seam)
    }

    /// List all registered seams.
    ///
    /// # Panics
    /// Panics if the internal seam registry lock is poisoned.
    pub fn list_seams(&self) -> Vec<SeamId> {
        let seams = self.seams.read().unwrap();
        seams.list()
    }

    /// Subscribe to an event.
    ///
    /// # Panics
    /// Panics if the internal event map lock is poisoned.
    pub fn on(&self, event_name: &str, handler: crate::event::EventHandler) {
        let mut events = self.events.write().unwrap();
        events.subscribe(event_name, handler);
    }

    /// Emit an event to all subscribers.
    ///
    /// # Panics
    /// Panics if the internal event map lock is poisoned.
    pub fn emit(&self, event: &crate::event::Event) {
        let events = self.events.read().unwrap();
        events.emit(event);
    }

    /// Get a configuration value by path (e.g. "`llm.default_model`").
    ///
    /// # Panics
    /// Panics if the internal configuration lock is poisoned.
    pub fn config_get(&self, path: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let guard = self.config.read().unwrap();
        let mut current = &*guard;
        for part in &parts {
            if let serde_json::Value::Object(map) = current {
                if let Some(v) = map.get(*part) {
                    current = v;
                    continue;
                }
            }
            return None;
        }
        Some(current.clone())
    }

    /// Set the full configuration.
    ///
    /// # Panics
    /// Panics if the internal configuration lock is poisoned.
    pub fn set_config(&self, config: serde_json::Value) {
        let mut cfg = self.config.write().unwrap();
        *cfg = config;
    }

    /// Deep-merge a configuration patch into the current configuration.
    ///
    /// Object values are merged recursively (the patch's keys override or add
    /// to the base); any non-object value in the patch replaces the
    /// corresponding base value. This is how profile `patch` overrides are
    /// applied: `load_profile` merges the profile's patch on top of the
    /// existing config so `config_get` resolves real values.
    ///
    /// # Panics
    /// Panics if the internal configuration lock is poisoned.
    pub fn apply_patch(&self, patch: &serde_json::Value) {
        let mut cfg = self.config.write().unwrap();
        *cfg = deep_merge(&cfg, patch);
    }

    /// Set the active profile.
    ///
    /// # Panics
    /// Panics if the internal profile lock is poisoned.
    pub fn set_profile(&self, profile: Profile) {
        let mut p = self.profile.write().unwrap();
        *p = profile;
    }

    /// Get the active profile name.
    ///
    /// # Panics
    /// Panics if the internal profile lock is poisoned.
    pub fn profile_name(&self) -> String {
        let p = self.profile.read().unwrap();
        p.name.clone()
    }

    /// Store an arbitrary value.
    ///
    /// # Panics
    /// Panics if the internal key-value store lock is poisoned.
    pub fn store_set(&self, key: &str, value: serde_json::Value) {
        let mut store = self.store.write().unwrap();
        store.insert(key.to_string(), value);
    }

    /// Retrieve a stored value.
    ///
    /// # Panics
    /// Panics if the internal key-value store lock is poisoned.
    pub fn store_get(&self, key: &str) -> Option<serde_json::Value> {
        let store = self.store.read().unwrap();
        store.get(key).cloned()
    }

    /// Dump the current state for debugging (like `dsh --dump-config`).
    ///
    /// # Panics
    /// Panics if any of the internal seams/config/profile locks is poisoned.
    pub fn dump(&self) -> serde_json::Value {
        let (seam_list, config, profile_name) = {
            let seams = self.seams.read().unwrap();
            let config = self.config.read().unwrap();
            let profile = self.profile.read().unwrap();
            (
                seams
                    .list()
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>(),
                config.clone(),
                profile.name.clone(),
            )
        };

        serde_json::json!({
            "profile": profile_name,
            "seams": seam_list,
            "config": config,
        })
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively merge `patch` into `base`.
///
/// When both sides are JSON objects, keys are merged (patch wins); otherwise
/// the patch value replaces the base value.
fn deep_merge(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(patch_map)) => {
            let mut merged = base_map.clone();
            for (key, value) in patch_map {
                let base_value = base_map.get(key).unwrap_or(&serde_json::Value::Null);
                merged.insert(key.clone(), deep_merge(base_value, value));
            }
            serde_json::Value::Object(merged)
        }
        (_, patch_value) => patch_value.clone(),
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

    #[test]
    fn ctx_get_seam_typed() {
        struct MarkerSeam {
            name: String,
        }
        impl Seam for MarkerSeam {
            fn name(&self) -> &str {
                &self.name
            }
        }

        let ctx = Ctx::new();
        let seam = SeamId::llm();
        let impl_arc = Arc::new(MarkerSeam {
            name: "marker".to_string(),
        }) as Arc<dyn Seam>;

        ctx.register_seam(seam.clone(), "marker-plugin".to_string(), impl_arc);

        let typed: Option<Arc<MarkerSeam>> = ctx.get_seam_typed(&seam);
        assert!(typed.is_some());
        assert_eq!(typed.unwrap().name(), "marker");

        // A different concrete type must not downcast.
        let wrong: Option<Arc<DummySeam>> = ctx.get_seam_typed(&seam);
        assert!(wrong.is_none());
    }

    #[test]
    fn ctx_apply_patch_deep_merges() {
        let ctx = Ctx::new();
        ctx.set_config(serde_json::json!({
            "llm": { "default_model": "a", "temperature": 0.7 },
            "ui": "cli"
        }));
        ctx.apply_patch(&serde_json::json!({
            "llm": { "default_model": "b" },
            "sandbox": { "timeout": 30 }
        }));

        // Patch overrides nested key.
        assert_eq!(
            ctx.config_get("llm.default_model"),
            Some(serde_json::json!("b"))
        );
        // Unpatched nested key is preserved.
        assert_eq!(
            ctx.config_get("llm.temperature"),
            Some(serde_json::json!(0.7))
        );
        // Untouched top-level key is preserved.
        assert_eq!(ctx.config_get("ui"), Some(serde_json::json!("cli")));
        // New top-level key is added.
        assert_eq!(
            ctx.config_get("sandbox.timeout"),
            Some(serde_json::json!(30))
        );
    }

    #[test]
    fn ctx_apply_patch_replaces_scalars() {
        let ctx = Ctx::new();
        ctx.set_config(serde_json::json!({"ui": "cli"}));
        ctx.apply_patch(&serde_json::json!({"ui": "web"}));
        assert_eq!(ctx.config_get("ui"), Some(serde_json::json!("web")));
    }
}

//! Plugin and bundle registries — resolve declared plugin/bundle names into
//! live plugin instances for automatic profile loading.
//!
//! A profile declares the plugins and bundles it wants via
//! [`pocker_core::types::Profile`]. The engine does not hard-code which plugins
//! exist; instead it asks these registries to turn a name into a concrete
//! [`pocker_core::plugin::Plugin`] instance. This keeps the engine generic and
//! lets applications (or the built-in [`crate::builtins`]) populate the set of
//! available plugins at startup.

use pocker_core::plugin::Plugin;
use pocker_core::types::PluginId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A factory that produces a fresh plugin instance for a given plugin name.
///
/// Plugins are instantiated lazily when a profile declares them, so the engine
/// never holds long-lived trait objects it is not actively using. The factory
/// must be `Send + Sync` because the engine is shared across async tasks.
pub type PluginFactory = Arc<dyn Fn() -> Arc<dyn Plugin> + Send + Sync>;

/// Resolves plugin names to live instances.
#[derive(Default)]
pub struct PluginRegistry {
    factories: RwLock<HashMap<String, PluginFactory>>,
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a factory for `name`. Re-registering overwrites the previous
    /// factory for that name.
    pub fn register(&self, name: &str, factory: PluginFactory) {
        self.factories
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(name.to_string(), factory);
    }

    /// Whether a factory is registered for `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.factories
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(name)
    }

    /// Instantiate the plugin named `name`, or `None` if no factory is known.
    #[must_use]
    pub fn create(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let factory = self
            .factories
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(name)
            .cloned()?;
        Some(factory())
    }

    /// List all registered plugin names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.factories
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }
}

/// Maps bundle names to the plugin ids they expand to.
///
/// A bundle is a named collection of plugins. Loading a bundle means loading
/// every plugin id it lists (bundles typically provide the base seams that
/// other plugins depend on, so they are mounted first).
#[derive(Default)]
pub struct BundleRegistry {
    bundles: RwLock<HashMap<String, Vec<PluginId>>>,
}

impl BundleRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the plugin ids that make up `name`. Re-registering overwrites.
    pub fn register(&self, name: &str, plugins: Vec<PluginId>) {
        self.bundles
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(name.to_string(), plugins);
    }

    /// Whether a bundle named `name` is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.bundles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(name)
    }

    /// Resolve `name` to its constituent plugin ids, or `None` if unknown.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Vec<PluginId>> {
        self.bundles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(name)
            .cloned()
    }

    /// List all registered bundle names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.bundles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }
}

/// The status of a single plugin after the automatic loading phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLoadStatus {
    /// Registered and mounted successfully.
    Loaded,
    /// Declared but never loaded (e.g. a missing bundle or missing factory).
    /// See `error`.
    Skipped,
    /// Registered but `mount` failed (e.g. a required seam was absent).
    /// See `error`.
    Failed,
}

/// Outcome of attempting to load one plugin during profile activation.
#[derive(Debug, Clone)]
pub struct PluginLoadOutcome {
    /// The plugin id as declared in the profile.
    pub id: PluginId,
    /// The logical name used in the loader (i.e. `id.name`).
    pub name: String,
    /// What happened to this plugin.
    pub status: PluginLoadStatus,
    /// Human-readable reason when `status` is `Skipped` or `Failed`.
    pub error: Option<String>,
}

/// Result of [`crate::runtime::Engine::load_profile`].
///
/// Carries the loaded [`pocker_core::types::Profile`] plus a per-plugin report
/// of the automatic loading phase so callers can surface partial failures
/// instead of the whole profile load failing on a single bad plugin.
#[derive(Debug, Clone)]
pub struct ProfileLoadResult {
    pub profile: pocker_core::types::Profile,
    pub outcomes: Vec<PluginLoadOutcome>,
}

impl ProfileLoadResult {
    /// Number of plugins that mounted successfully.
    #[must_use]
    pub fn loaded_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.status == PluginLoadStatus::Loaded)
            .count()
    }

    /// Number of plugins that were skipped or failed.
    #[must_use]
    pub fn problem_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.status != PluginLoadStatus::Loaded)
            .count()
    }
}

//! Pocker Engine runtime — the main entry point.
//!
//! The engine holds the shared context, the plugin loader, and the plugin /
//! bundle registries. It orchestrates profile loading, plugin mounting, and
//! lifecycle.

use crate::loader::PluginLoader;
use crate::profile::ProfileManager;
use crate::registry::{
    BundleRegistry, PluginFactory, PluginLoadOutcome, PluginLoadStatus, PluginRegistry,
    ProfileLoadResult,
};
use pocker_core::context::Ctx;
use pocker_core::error::{PockerError, Result};
use pocker_core::plugin::Plugin;
use pocker_core::types::{PluginId, Profile};
use std::sync::Arc;
use tracing::info;

/// The Pocker Engine. Holds the shared context and manages plugins.
pub struct Engine {
    /// Shared context — all plugins register here
    pub ctx: Arc<Ctx>,
    /// Plugin loader — manages plugin lifecycle
    pub loader: PluginLoader,
    /// Profile manager — handles profile files
    pub profiles: ProfileManager,
    /// Plugin factory registry (name -> instance factory) for auto-loading
    pub plugins: PluginRegistry,
    /// Bundle registry (bundle name -> constituent plugin ids)
    pub bundles: BundleRegistry,
    /// Current profile name
    current_profile: Arc<std::sync::RwLock<String>>,
}

impl Engine {
    /// Create a new engine with the default profile directory.
    #[must_use]
    pub fn new() -> Self {
        let profiles = match ProfileManager::new() {
            Ok(pm) => pm,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "home directory unavailable; falling back to a CWD-relative profile dir"
                );
                ProfileManager::default()
            }
        };
        let engine = Self {
            ctx: Arc::new(Ctx::new()),
            loader: PluginLoader::new(),
            profiles,
            plugins: PluginRegistry::new(),
            bundles: BundleRegistry::new(),
            current_profile: Arc::new(std::sync::RwLock::new("default".to_string())),
        };
        crate::builtins::register_defaults(&engine.plugins, &engine.bundles);
        engine
    }

    /// Create a new engine with a custom profile directory (for testing).
    pub fn with_profile_dir(dir: impl Into<std::path::PathBuf>) -> Self {
        let profiles = ProfileManager::with_base_dir(dir);
        let engine = Self {
            ctx: Arc::new(Ctx::new()),
            loader: PluginLoader::new(),
            profiles,
            plugins: PluginRegistry::new(),
            bundles: BundleRegistry::new(),
            current_profile: Arc::new(std::sync::RwLock::new("default".to_string())),
        };
        crate::builtins::register_defaults(&engine.plugins, &engine.bundles);
        engine
    }

    /// Get the shared context.
    pub const fn context(&self) -> &Arc<Ctx> {
        &self.ctx
    }

    /// Register a plugin (does not mount).
    ///
    /// # Errors
    /// Propagates the error from [`crate::loader::PluginLoader::register`].
    pub fn register_plugin(&self, name: &str, plugin: Arc<dyn Plugin>) -> Result<()> {
        self.loader.register(name, plugin)
    }

    /// Register a plugin factory so that profiles declaring this plugin name
    /// can be auto-loaded by [`Engine::load_profile`].
    pub fn register_plugin_factory(&self, name: &str, factory: PluginFactory) {
        self.plugins.register(name, factory);
    }

    /// Register a bundle name -> constituent plugin ids mapping, so that
    /// profiles declaring the bundle are auto-expanded during loading.
    pub fn register_bundle(&self, name: &str, ids: Vec<PluginId>) {
        self.bundles.register(name, ids);
    }

    /// Mount a specific plugin.
    ///
    /// # Errors
    /// Propagates the error from [`crate::loader::PluginLoader::mount`].
    pub async fn mount_plugin(&self, name: &str) -> Result<()> {
        self.loader.mount(name, &self.ctx).await
    }

    /// Unmount a specific plugin.
    ///
    /// # Errors
    /// Propagates the error from [`crate::loader::PluginLoader::unmount`].
    pub async fn unmount_plugin(&self, name: &str) -> Result<()> {
        self.loader.unmount(name, &self.ctx).await
    }

    /// Unmount all plugins (cleanup).
    ///
    /// # Errors
    /// Never fails: delegates to
    /// [`crate::loader::PluginLoader::unmount_all`], which swallows
    /// per-plugin errors.
    pub async fn shutdown(&self) -> Result<()> {
        info!("engine shutting down");
        self.loader.unmount_all(&self.ctx).await
    }

    /// Get the current profile name.
    ///
    /// # Panics
    /// Panics if the internal current-profile lock is poisoned.
    pub fn current_profile(&self) -> String {
        self.current_profile.read().unwrap().clone()
    }

    /// Load and activate a profile.
    ///
    /// Beyond reading the profile file and merging its config patch, this now
    /// *automatically loads and initializes* every plugin and bundle the
    /// profile declares (see [`Engine::activate_plugins`]). This replaces the
    /// previous placeholder behaviour where the profile merely declared
    /// plugins without ever loading them.
    ///
    /// # Errors
    /// Returns [`pocker_core::error::PockerError::Config`] if the profile file
    /// cannot be read or parsed. Individual plugin load failures do **not**
    /// abort the profile: they are reported in
    /// [`ProfileLoadResult::outcomes`].
    ///
    /// # Panics
    /// Panics if the internal current-profile lock is poisoned.
    pub async fn load_profile(&self, name: &str) -> Result<ProfileLoadResult> {
        let profile = self
            .profiles
            .load(name)
            .map_err(|e| PockerError::Config(e.to_string()))?;
        // Apply the profile's config patch on top of the current config so
        // `ctx.config_get` resolves real values (e.g. llm.default_model).
        self.ctx.apply_patch(&profile.patch);
        self.ctx.set_profile(profile.clone());
        *self.current_profile.write().unwrap() = name.to_string();
        info!(profile = name, "profile loaded");
        // Automatically load and initialize the plugins/bundles the profile
        // declares, with dependency ordering and per-plugin error handling.
        let outcomes = self.activate_plugins(&profile).await;
        Ok(ProfileLoadResult { profile, outcomes })
    }

    /// Expand the profile's bundles + plugins into a deduplicated load plan,
    /// then instantiate, register, and mount each plugin in dependency order.
    ///
    /// The phase is resilient: a single bad plugin (missing factory, missing
    /// bundle, or mount failure) is recorded in [`PluginLoadOutcome`] and does
    /// not abort the rest of the profile.
    async fn activate_plugins(&self, profile: &Profile) -> Vec<PluginLoadOutcome> {
        let mut outcomes: Vec<PluginLoadOutcome> = Vec::new();

        // 1. Build the ordered plan: bundles first (they provide base seams),
        //    then the profile's direct plugins. Deduplicate by plugin name.
        let mut plan: Vec<PluginId> = Vec::new();

        for bundle_name in &profile.bundles {
            match self.bundles.resolve(bundle_name) {
                Some(ids) => {
                    for id in ids {
                        if !plan.iter().any(|p| p.name == id.name) {
                            plan.push(id);
                        }
                    }
                }
                None => outcomes.push(PluginLoadOutcome {
                    id: PluginId::new(bundle_name.clone(), "0.0.0"),
                    name: bundle_name.clone(),
                    status: PluginLoadStatus::Skipped,
                    error: Some(format!("bundle not registered: {bundle_name}")),
                }),
            }
        }

        for id in &profile.plugins {
            if !plan.iter().any(|p| p.name == id.name) {
                plan.push(id.clone());
            }
        }

        // 2. Instantiate every plan entry up front (construction is
        //    side-effect free) so we can read metadata for dependency ordering.
        let mut instances: Vec<(PluginId, Arc<dyn Plugin>)> = Vec::new();
        for id in &plan {
            match self.plugins.create(&id.name) {
                Some(plugin) => instances.push((id.clone(), plugin)),
                None => outcomes.push(PluginLoadOutcome {
                    id: id.clone(),
                    name: id.name.clone(),
                    status: PluginLoadStatus::Skipped,
                    error: Some(format!("no plugin factory registered for '{}'", id.name)),
                }),
            }
        }

        // 3. Topological order by seam provides/requires (providers before
        //    consumers).
        let ordered = order_by_dependencies(&instances);

        // 4. Register + mount in dependency order, recording each outcome.
        for id in ordered {
            let plugin = instances
                .iter()
                .find(|(pid, _)| pid.name == id.name)
                .map(|(_, p)| p.clone())
                .expect("ordered id must have a corresponding instance");

            if let Err(e) = self.loader.register(&id.name, plugin) {
                outcomes.push(PluginLoadOutcome {
                    id: id.clone(),
                    name: id.name.clone(),
                    status: PluginLoadStatus::Failed,
                    error: Some(format!("register failed: {e}")),
                });
                continue;
            }

            match self.loader.mount(&id.name, &self.ctx).await {
                Ok(()) => outcomes.push(PluginLoadOutcome {
                    id: id.clone(),
                    name: id.name.clone(),
                    status: PluginLoadStatus::Loaded,
                    error: None,
                }),
                Err(e) => outcomes.push(PluginLoadOutcome {
                    id: id.clone(),
                    name: id.name.clone(),
                    status: PluginLoadStatus::Failed,
                    error: Some(format!("mount failed: {e}")),
                }),
            }
        }

        outcomes
    }

    /// List all available profiles.
    ///
    /// # Errors
    /// Returns [`pocker_core::error::PockerError::Config`] if the profile
    /// directory cannot be enumerated.
    pub fn list_profiles(&self) -> Result<Vec<String>> {
        self.profiles
            .list()
            .map_err(|e| PockerError::Config(e.to_string()))
    }

    /// Dump the engine state (for `pocker --dump-config`).
    ///
    /// # Errors
    /// This function does not return a `Result`; it always succeeds.
    pub fn dump(&self) -> serde_json::Value {
        let plugins = self.loader.list();
        serde_json::json!({
            "engine": {
                "profile": self.current_profile(),
                "context": self.ctx.dump(),
                "plugins": plugins.iter().map(|(name, mounted)| {
                    serde_json::json!({
                        "name": name,
                        "mounted": mounted,
                    })
                }).collect::<Vec<_>>(),
            }
        })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Order plugin ids so that any plugin providing a seam is mounted before any
/// plugin requiring that seam (Kahn's algorithm over seam dependencies).
///
/// Ties are broken by the original plan index, which keeps bundle plugins
/// ahead of directly-declared plugins. Unresolvable cycles are broken
/// deterministically (smallest original index) so loading never deadlocks.
fn order_by_dependencies(instances: &[(PluginId, Arc<dyn Plugin>)]) -> Vec<PluginId> {
    use std::collections::{HashMap, HashSet};

    let provides: HashMap<String, HashSet<String>> = instances
        .iter()
        .map(|(id, p)| {
            (
                id.name.clone(),
                p.metadata().provides.iter().cloned().collect(),
            )
        })
        .collect();
    let requires: HashMap<String, Vec<String>> = instances
        .iter()
        .map(|(id, p)| (id.name.clone(), p.metadata().requires.clone()))
        .collect();
    let index: HashMap<String, usize> = instances
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.name.clone(), i))
        .collect();

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut indegree: HashMap<String, usize> = HashMap::new();
    for (name, _) in instances {
        adj.entry(name.name.clone()).or_default();
        indegree.entry(name.name.clone()).or_insert(0);
    }

    for (consumer, reqs) in &requires {
        for seam in reqs {
            for (provider, prov_seams) in &provides {
                if provider != consumer && prov_seams.contains(seam) {
                    adj.get_mut(provider).unwrap().push(consumer.clone());
                    *indegree.get_mut(consumer).unwrap() += 1;
                }
            }
        }
    }

    let mut remaining: HashSet<String> = instances.iter().map(|(id, _)| id.name.clone()).collect();
    let mut ordered: Vec<PluginId> = Vec::new();
    while !remaining.is_empty() {
        let mut ready: Vec<String> = remaining
            .iter()
            .filter(|&n| indegree.get(n).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        if ready.is_empty() {
            // Cycle: break it by taking the smallest-index remaining node.
            let next = remaining
                .iter()
                .min_by_key(|n| index.get(*n).copied())
                .cloned()
                .expect("remaining is non-empty");
            ready.push(next);
        }
        ready.sort_by_key(|n| index.get(n).copied());
        let n = ready.remove(0);
        remaining.remove(&n);
        let pid = instances
            .iter()
            .find(|(id, _)| id.name == n)
            .unwrap()
            .0
            .clone();
        ordered.push(pid);
        if let Some(neighbors) = adj.get(&n) {
            for nb in neighbors {
                if let Some(d) = indegree.get_mut(nb) {
                    *d -= 1;
                }
            }
        }
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocker_core::plugin::PluginMetadata;
    use tempfile::tempdir;

    struct NoopPlugin {
        meta: PluginMetadata,
    }

    #[async_trait::async_trait]
    impl Plugin for NoopPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
        async fn mount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            Ok(())
        }
        async fn unmount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn engine_register_and_mount() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        let plugin = Arc::new(NoopPlugin {
            meta: PluginMetadata::new("noop", "1.0.0"),
        });

        engine.register_plugin("noop", plugin).unwrap();
        engine.mount_plugin("noop").await.unwrap();

        let dump = engine.dump();
        assert_eq!(dump["engine"]["plugins"][0]["name"], "noop");
        assert_eq!(dump["engine"]["plugins"][0]["mounted"], true);
    }

    #[tokio::test]
    async fn engine_shutdown_unmounts_all() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        let p1 = Arc::new(NoopPlugin {
            meta: PluginMetadata::new("p1", "1.0.0"),
        });
        let p2 = Arc::new(NoopPlugin {
            meta: PluginMetadata::new("p2", "1.0.0"),
        });

        engine.register_plugin("p1", p1).unwrap();
        engine.register_plugin("p2", p2).unwrap();
        engine.mount_plugin("p1").await.unwrap();
        engine.mount_plugin("p2").await.unwrap();

        engine.shutdown().await.unwrap();

        assert!(!engine.loader.is_mounted("p1"));
        assert!(!engine.loader.is_mounted("p2"));
    }

    #[tokio::test]
    async fn engine_profile_management() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        // Create a profile
        engine
            .profiles
            .create("test", "Test", vec!["@pocker/core".to_string()])
            .unwrap();

        // List profiles
        let profiles = engine.list_profiles().unwrap();
        assert!(profiles.contains(&"test".to_string()));

        // Load profile
        let result = engine.load_profile("test").await.unwrap();
        assert_eq!(result.profile.name, "test");
        assert_eq!(engine.current_profile(), "test");
    }

    #[tokio::test]
    async fn engine_load_profile_applies_patch() {
        let tmp = tempdir().unwrap();
        let pm = ProfileManager::with_base_dir(tmp.path());
        let profile = Profile {
            name: "cfg".to_string(),
            description: "cfg".to_string(),
            bundles: Vec::new(),
            plugins: Vec::new(),
            patch: serde_json::json!({ "llm": { "default_model": "deepseek-chat" } }),
        };
        pm.save(&profile).unwrap();

        let engine = Engine::with_profile_dir(tmp.path());
        engine.load_profile("cfg").await.unwrap();

        // The profile's config patch must be merged into the context config.
        assert_eq!(
            engine.ctx.config_get("llm.default_model"),
            Some(serde_json::json!("deepseek-chat"))
        );
    }

    // --- Auto-loading behaviour -------------------------------------------

    struct ProviderPlugin {
        meta: PluginMetadata,
    }

    #[async_trait::async_trait]
    impl Plugin for ProviderPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
        async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()> {
            ctx.register_seam(
                pocker_core::seam::SeamId::new("ctx.foo"),
                self.meta.name.clone(),
                self.meta.digest(),
                Arc::new(NoopSeam {
                    name: "foo".to_string(),
                }) as Arc<dyn pocker_core::seam::Seam>,
            );
            Ok(())
        }
        async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()> {
            ctx.unregister_seam(&pocker_core::seam::SeamId::new("ctx.foo"), &self.meta.name);
            Ok(())
        }
    }

    struct ConsumerPlugin {
        meta: PluginMetadata,
    }

    #[async_trait::async_trait]
    impl Plugin for ConsumerPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
        async fn mount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            Ok(())
        }
        async fn unmount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            Ok(())
        }
    }

    struct NoopSeam {
        name: String,
    }
    impl pocker_core::seam::Seam for NoopSeam {
        fn name(&self) -> &str {
            &self.name
        }
    }

    fn provider_meta(name: &str) -> PluginMetadata {
        let mut m = PluginMetadata::new(name, "1.0.0");
        m.provides = vec!["ctx.foo".to_string()];
        m
    }

    fn consumer_meta(name: &str) -> PluginMetadata {
        let mut m = PluginMetadata::new(name, "1.0.0");
        m.requires = vec!["ctx.foo".to_string()];
        m
    }

    #[tokio::test]
    async fn load_profile_autoloads_bundle_and_plugins() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        // Register a custom plugin + a bundle that includes it.
        engine.register_plugin_factory(
            "demo",
            Arc::new(|| {
                Arc::new(ProviderPlugin {
                    meta: provider_meta("demo"),
                })
            }),
        );
        engine.register_bundle(
            "demo-bundle",
            vec![PluginId::new("demo", "1.0.0")],
        );

        let profile = Profile {
            name: "p".to_string(),
            description: "".to_string(),
            bundles: vec!["demo-bundle".to_string()],
            plugins: vec![],
            patch: serde_json::Value::Null,
        };
        engine.profiles.save(&profile).unwrap();

        let result = engine.load_profile("p").await.unwrap();
        assert_eq!(result.loaded_count(), 1);
        assert!(result.problem_count() == 0);
        assert!(engine.loader.is_mounted("demo"));
        // The bundle's seam should now be present in the context.
        assert!(engine
            .ctx
            .has_seam(&pocker_core::seam::SeamId::new("ctx.foo")));
    }

    #[tokio::test]
    async fn load_profile_orders_dependencies() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        engine.register_plugin_factory(
            "provider",
            Arc::new(|| Arc::new(ProviderPlugin { meta: provider_meta("provider") })),
        );
        engine.register_plugin_factory(
            "consumer",
            Arc::new(|| Arc::new(ConsumerPlugin { meta: consumer_meta("consumer") })),
        );

        // Declared out of order (consumer before provider) to prove ordering
        // is driven by seam dependencies, not declaration order.
        let profile = Profile {
            name: "dep".to_string(),
            description: "".to_string(),
            bundles: vec![],
            plugins: vec![
                PluginId::new("consumer", "1.0.0"),
                PluginId::new("provider", "1.0.0"),
            ],
            patch: serde_json::Value::Null,
        };
        engine.profiles.save(&profile).unwrap();

        let result = engine.load_profile("dep").await.unwrap();
        assert_eq!(result.loaded_count(), 2, "both plugins should load");
        assert!(engine.loader.is_mounted("provider"));
        assert!(engine.loader.is_mounted("consumer"));
    }

    #[tokio::test]
    async fn load_profile_reports_missing_bundle_and_factory() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());
        // No factory registered for "ghost"; no bundle registered for "ghost-bundle".

        let profile = Profile {
            name: "bad".to_string(),
            description: "".to_string(),
            bundles: vec!["ghost-bundle".to_string()],
            plugins: vec![PluginId::new("ghost", "1.0.0")],
            patch: serde_json::Value::Null,
        };
        engine.profiles.save(&profile).unwrap();

        let result = engine.load_profile("bad").await.unwrap();
        // Two declared entries, both should be reported as skipped (not loaded).
        assert_eq!(result.loaded_count(), 0);
        assert_eq!(result.problem_count(), 2);
        assert!(result
            .outcomes
            .iter()
            .any(|o| o.name == "ghost-bundle" && o.status == PluginLoadStatus::Skipped));
        assert!(result
            .outcomes
            .iter()
            .any(|o| o.name == "ghost" && o.status == PluginLoadStatus::Skipped));
    }

    #[tokio::test]
    async fn load_profile_reports_mount_failure() {
        let tmp = tempdir().unwrap();
        let engine = Engine::with_profile_dir(tmp.path());

        // A plugin that requires a seam nothing provides -> mount fails.
        engine.register_plugin_factory(
            "needy",
            Arc::new(|| Arc::new(ConsumerPlugin { meta: consumer_meta("needy") })),
        );

        let profile = Profile {
            name: "need".to_string(),
            description: "".to_string(),
            bundles: vec![],
            plugins: vec![PluginId::new("needy", "1.0.0")],
            patch: serde_json::Value::Null,
        };
        engine.profiles.save(&profile).unwrap();

        let result = engine.load_profile("need").await.unwrap();
        assert_eq!(result.loaded_count(), 0);
        assert!(result
            .outcomes
            .iter()
            .any(|o| o.name == "needy" && o.status == PluginLoadStatus::Failed));
    }
}

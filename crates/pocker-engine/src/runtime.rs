//! Pocker Engine runtime — the main entry point.
//!
//! The engine holds the shared context and plugin loader.
//! It orchestrates profile loading, plugin mounting, and lifecycle.

use crate::loader::PluginLoader;
use crate::profile::ProfileManager;
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use pocker_core::plugin::Plugin;
use pocker_core::types::Profile;
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
    /// Current profile name
    current_profile: Arc<std::sync::RwLock<String>>,
}

impl Engine {
    /// Create a new engine with the default profile directory.
    pub fn new() -> Self {
        let profiles = ProfileManager::new().unwrap_or_default();
        Self {
            ctx: Arc::new(Ctx::new()),
            loader: PluginLoader::new(),
            profiles,
            current_profile: Arc::new(std::sync::RwLock::new("default".to_string())),
        }
    }

    /// Create a new engine with a custom profile directory (for testing).
    pub fn with_profile_dir(dir: impl Into<std::path::PathBuf>) -> Self {
        let profiles = ProfileManager::with_base_dir(dir);
        Self {
            ctx: Arc::new(Ctx::new()),
            loader: PluginLoader::new(),
            profiles,
            current_profile: Arc::new(std::sync::RwLock::new("default".to_string())),
        }
    }

    /// Get the shared context.
    pub fn context(&self) -> &Arc<Ctx> {
        &self.ctx
    }

    /// Register a plugin (does not mount).
    pub fn register_plugin(&self, name: &str, plugin: Arc<dyn Plugin>) -> Result<()> {
        self.loader.register(name, plugin)
    }

    /// Mount a specific plugin.
    pub async fn mount_plugin(&self, name: &str) -> Result<()> {
        self.loader.mount(name, &self.ctx).await
    }

    /// Unmount a specific plugin.
    pub async fn unmount_plugin(&self, name: &str) -> Result<()> {
        self.loader.unmount(name, &self.ctx).await
    }

    /// Unmount all plugins (cleanup).
    pub async fn shutdown(&self) -> Result<()> {
        info!("engine shutting down");
        self.loader.unmount_all(&self.ctx).await
    }

    /// Get the current profile name.
    pub fn current_profile(&self) -> String {
        self.current_profile.read().unwrap().clone()
    }

    /// Load and activate a profile.
    pub fn load_profile(&self, name: &str) -> Result<Profile> {
        let profile = self
            .profiles
            .load(name)
            .map_err(|e| pocker_core::error::PockerError::Config(e.to_string()))?;
        self.ctx.set_profile(profile.clone());
        *self.current_profile.write().unwrap() = name.to_string();
        info!(profile = name, "profile loaded");
        Ok(profile)
    }

    /// List all available profiles.
    pub fn list_profiles(&self) -> Result<Vec<String>> {
        self.profiles
            .list()
            .map_err(|e| pocker_core::error::PockerError::Config(e.to_string()))
    }

    /// Dump the engine state (for `pocker --dump-config`).
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
        assert_eq!(
            dump["engine"]["plugins"][0]["name"],
            "noop"
        );
        assert_eq!(
            dump["engine"]["plugins"][0]["mounted"],
            true
        );
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

    #[test]
    fn engine_profile_management() {
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
        let profile = engine.load_profile("test").unwrap();
        assert_eq!(profile.name, "test");
        assert_eq!(engine.current_profile(), "test");
    }
}

//! Plugin loader — manages plugin lifecycle (mount/unmount).

use pocker_core::context::Ctx;
use pocker_core::error::{PockerError, Result};
use pocker_core::plugin::{Plugin, PluginHandle};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::info;

/// The plugin loader manages plugin lifecycle.
/// It tracks all loaded plugins and their states.
pub struct PluginLoader {
    plugins: RwLock<HashMap<String, PluginHandle>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin (does not mount it yet).
    pub fn register(&self, name: &str, plugin: Arc<dyn Plugin>) -> Result<()> {
        let handle = PluginHandle::new(plugin);
        let mut plugins = self.plugins.write().map_err(|e| {
            PockerError::Plugin(format!("lock poisoned: {e}"))
        })?;
        plugins.insert(name.to_string(), handle);
        info!(plugin = name, "plugin registered");
        Ok(())
    }

    /// Mount a plugin into the context.
    pub async fn mount(&self, name: &str, ctx: &Arc<Ctx>) -> Result<()> {
        let plugin = {
            let plugins = self.plugins.read().map_err(|e| {
                PockerError::Plugin(format!("lock poisoned: {e}"))
            })?;
            plugins
                .get(name)
                .ok_or_else(|| PockerError::Plugin(format!("plugin not found: {name}")))?
                .plugin
                .clone()
        };

        plugin.mount(ctx).await?;

        {
            let plugins = self.plugins.read().map_err(|e| {
                PockerError::Plugin(format!("lock poisoned: {e}"))
            })?;
            if let Some(handle) = plugins.get(name) {
                handle.set_mounted(true);
            }
        }

        info!(plugin = name, "plugin mounted");
        Ok(())
    }

    /// Unmount a plugin from the context.
    pub async fn unmount(&self, name: &str, ctx: &Arc<Ctx>) -> Result<()> {
        let plugin = {
            let plugins = self.plugins.read().map_err(|e| {
                PockerError::Plugin(format!("lock poisoned: {e}"))
            })?;
            plugins
                .get(name)
                .ok_or_else(|| PockerError::Plugin(format!("plugin not found: {name}")))?
                .plugin
                .clone()
        };

        plugin.unmount(ctx).await?;

        {
            let plugins = self.plugins.read().map_err(|e| {
                PockerError::Plugin(format!("lock poisoned: {e}"))
            })?;
            if let Some(handle) = plugins.get(name) {
                handle.set_mounted(false);
            }
        }

        info!(plugin = name, "plugin unmounted");
        Ok(())
    }

    /// Unmount all mounted plugins (in reverse order of mounting).
    pub async fn unmount_all(&self, ctx: &Arc<Ctx>) -> Result<()> {
        let names: Vec<String> = {
            let plugins = self.plugins.read().map_err(|e| {
                PockerError::Plugin(format!("lock poisoned: {e}"))
            })?;
            plugins
                .iter()
                .filter(|(_, h)| h.is_mounted())
                .map(|(k, _)| k.clone())
                .collect()
        };

        for name in names.into_iter().rev() {
            if let Err(e) = self.unmount(&name, ctx).await {
                tracing::warn!(plugin = %name, error = %e, "failed to unmount plugin");
            }
        }
        Ok(())
    }

    /// List all registered plugins with their mount state.
    pub fn list(&self) -> Vec<(String, bool)> {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins
            .iter()
            .map(|(name, handle)| (name.clone(), handle.is_mounted()))
            .collect()
    }

    /// Check if a plugin is mounted.
    pub fn is_mounted(&self, name: &str) -> bool {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins
            .get(name)
            .map(|h| h.is_mounted())
            .unwrap_or(false)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocker_core::plugin::PluginMetadata;
    use pocker_core::types::PluginType;

    struct DummyPlugin {
        meta: PluginMetadata,
        mounted: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Plugin for DummyPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }
        async fn mount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            self.mounted.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
        async fn unmount(&self, _ctx: &Arc<Ctx>) -> Result<()> {
            self.mounted.store(false, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn loader_register_and_mount() {
        let loader = PluginLoader::new();
        let ctx = Arc::new(Ctx::new());
        let mounted = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let plugin = Arc::new(DummyPlugin {
            meta: {
                let mut m = PluginMetadata::new("dummy", "1.0.0");
                m.plugin_type = PluginType::Other;
                m
            },
            mounted: mounted.clone(),
        });

        loader.register("dummy", plugin).unwrap();
        assert!(!loader.is_mounted("dummy"));

        loader.mount("dummy", &ctx).await.unwrap();
        assert!(loader.is_mounted("dummy"));
        assert!(mounted.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn loader_unmount() {
        let loader = PluginLoader::new();
        let ctx = Arc::new(Ctx::new());
        let mounted = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let plugin = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("dummy", "1.0.0"),
            mounted: mounted.clone(),
        });

        loader.register("dummy", plugin).unwrap();
        loader.mount("dummy", &ctx).await.unwrap();
        assert!(loader.is_mounted("dummy"));

        loader.unmount("dummy", &ctx).await.unwrap();
        assert!(!loader.is_mounted("dummy"));
        assert!(!mounted.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn loader_list() {
        let loader = PluginLoader::new();
        let ctx = Arc::new(Ctx::new());

        let p1 = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("p1", "1.0.0"),
            mounted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        let p2 = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("p2", "1.0.0"),
            mounted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });

        loader.register("p1", p1).unwrap();
        loader.register("p2", p2).unwrap();

        loader.mount("p1", &ctx).await.unwrap();

        let list = loader.list();
        assert_eq!(list.len(), 2);
        let p1_entry = list.iter().find(|(n, _)| n == "p1").unwrap();
        assert!(p1_entry.1);
        let p2_entry = list.iter().find(|(n, _)| n == "p2").unwrap();
        assert!(!p2_entry.1);
    }

    #[tokio::test]
    async fn loader_mount_nonexistent_fails() {
        let loader = PluginLoader::new();
        let ctx = Arc::new(Ctx::new());
        let result = loader.mount("nonexistent", &ctx).await;
        assert!(result.is_err());
    }
}

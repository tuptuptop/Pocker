//! Testing utilities for plugin developers.

use pocker_core::context::Ctx;
use pocker_core::plugin::Plugin;
use std::sync::Arc;

/// Test harness for plugins — provides a context and handles mount/unmount.
pub struct PluginTestHarness {
    pub ctx: Arc<Ctx>,
}

impl PluginTestHarness {
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(Ctx::new()),
        }
    }

    /// Mount a plugin and return a guard that unmounts on drop.
    pub async fn mount(&self, plugin: &Arc<dyn Plugin>) -> anyhow::Result<MountGuard<'_>> {
        plugin.mount(&self.ctx).await?;
        Ok(MountGuard {
            ctx: &self.ctx,
            plugin: plugin.clone(),
        })
    }
}

impl Default for PluginTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that automatically unmounts a plugin when dropped.
pub struct MountGuard<'a> {
    ctx: &'a Arc<Ctx>,
    plugin: Arc<dyn Plugin>,
}

impl Drop for MountGuard<'_> {
    fn drop(&mut self) {
        // Fire and forget unmount
        let ctx = self.ctx.clone();
        let plugin = self.plugin.clone();
        tokio::spawn(async move {
            let _ = plugin.unmount(&ctx).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocker_core::error::Result;
    use pocker_core::plugin::PluginMetadata;

    struct DummyPlugin {
        meta: PluginMetadata,
    }

    #[async_trait::async_trait]
    impl Plugin for DummyPlugin {
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
    async fn harness_mount_and_auto_unmount() {
        let harness = PluginTestHarness::new();
        let plugin: Arc<dyn Plugin> = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("test", "1.0.0"),
        });

        {
            let _guard = harness.mount(&plugin).await.unwrap();
            // Plugin is mounted here
        }
        // Guard dropped — unmount fired in background
        // Give it a moment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

//! Plugin trait and metadata.
//!
//! Every part of Pocker is a plugin. Plugins mount into the shared context,
//! register their services on seams, and automatically unwind on unmount.

use crate::context::Ctx;
use crate::error::Result;
use crate::types::PluginType;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique name (e.g. "@pocker/llm-openai")
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Seams this plugin provides
    pub provides: Vec<String>,
    /// Seams this plugin requires (dependencies)
    pub requires: Vec<String>,
}

impl PluginMetadata {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            plugin_type: PluginType::Other,
            provides: Vec::new(),
            requires: Vec::new(),
        }
    }
}

/// Runtime state of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Created but not yet mounted
    Created,
    /// Currently mounting
    Mounting,
    /// Mounted and active
    Mounted,
    /// Currently unmounting
    Unmounting,
    /// Unmounted (can be remounted)
    Unmounted,
    /// Failed to mount
    Failed,
}

/// A plugin. Plugins contribute services, events, and reversible effects
/// to the shared context.
///
/// The `mount` method registers the plugin's services on seams.
/// The `unmount` method reverses those registrations (unwinds effects).
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Metadata about this plugin.
    fn metadata(&self) -> &PluginMetadata;

    /// Mount this plugin into the shared context.
    /// Register services, subscribe to events, etc.
    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()>;

    /// Unmount this plugin from the shared context.
    /// All registrations made in `mount` should be reversed.
    async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()>;
}

/// A handle to a loaded plugin, tracking its state.
pub struct PluginHandle {
    pub plugin: Arc<dyn Plugin>,
    pub state: Arc<AtomicBool>, // true = mounted
}

impl PluginHandle {
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            state: Arc::new(AtomicBool::new(false)),
        }
    }

    #[must_use]
    pub fn is_mounted(&self) -> bool {
        self.state.load(Ordering::SeqCst)
    }

    pub fn set_mounted(&self, mounted: bool) {
        self.state.store(mounted, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_new() {
        let meta = PluginMetadata::new("test-plugin", "1.0.0");
        assert_eq!(meta.name, "test-plugin");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.plugin_type, PluginType::Other);
    }

    #[test]
    fn plugin_handle_state() {
        struct DummyPlugin {
            meta: PluginMetadata,
        }

        #[async_trait]
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

        let plugin = Arc::new(DummyPlugin {
            meta: PluginMetadata::new("dummy", "0.1.0"),
        });
        let handle = PluginHandle::new(plugin);

        assert!(!handle.is_mounted());
        handle.set_mounted(true);
        assert!(handle.is_mounted());
    }
}

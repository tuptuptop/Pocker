//! Pocker Engine — the runtime that loads plugins, manages profiles,
//! and orchestrates the plugin tree.
//!
//! The engine has no privileged core. It is merely a loader and orchestrator
//! that composes plugins according to the active profile.

#[cfg(test)]
mod integration;
pub mod builtins;
pub mod loader;
pub mod profile;
pub mod registry;
pub mod runtime;

pub use builtins::CoreBundlePlugin;
pub use loader::PluginLoader;
pub use profile::{ProfileError, ProfileManager};
pub use registry::{
    BundleRegistry, PluginFactory, PluginLoadOutcome, PluginLoadStatus, PluginRegistry,
    ProfileLoadResult,
};
pub use runtime::Engine;

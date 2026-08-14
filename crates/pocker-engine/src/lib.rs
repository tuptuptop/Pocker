//! Pocker Engine — the runtime that loads plugins, manages profiles,
//! and orchestrates the plugin tree.
//!
//! The engine has no privileged core. It is merely a loader and orchestrator
//! that composes plugins according to the active profile.

pub mod loader;
pub mod profile;
pub mod runtime;

pub use loader::PluginLoader;
pub use profile::{ProfileManager, ProfileError};
pub use runtime::Engine;

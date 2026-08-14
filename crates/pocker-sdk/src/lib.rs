//! Pocker SDK — utilities for plugin developers.
//!
//! Provides helper types and macros for building Pocker plugins.

pub mod macros;
pub mod testing;

pub use pocker_core::plugin::{Plugin, PluginMetadata};
pub use pocker_plugin::{LlmAdapter, Skill, Tool};

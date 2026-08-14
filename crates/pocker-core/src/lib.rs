//! Pocker Core — Shared types, context, and event system.
//!
//! This crate provides the foundational types and the shared context (`Ctx`)
//! that all plugins register their services into. There is no privileged core;
//! everything is a plugin.

pub mod context;
pub mod error;
pub mod event;
pub mod plugin;
pub mod seam;
pub mod types;

pub use context::Ctx;
pub use error::{PockerError, Result};
pub use event::{Event, EventHandler, EventMap};
pub use plugin::{Plugin, PluginMetadata, PluginState};
pub use seam::{Seam, SeamId};
pub use types::*;

//! Pocker Plugin SDK — plugin traits, default implementations, and registry.
//!
//! This crate provides the building blocks for creating Pocker plugins.

pub mod llm;
pub mod skill;
pub mod tool;

pub use llm::{LlmAdapter, LlmSeam};
pub use skill::{Skill, SkillSeam};
pub use tool::{Tool, ToolSeam, ToolResult};

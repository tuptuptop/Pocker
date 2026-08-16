//! Pocker Plugin SDK — plugin traits, default implementations, and registry.
//!
//! This crate provides the building blocks for creating Pocker plugins.

pub mod llm;
pub mod prompt;
pub mod skill;
pub mod tool;

pub use llm::{LlmAdapter, LlmSeam};
pub use prompt::PromptSeam;
pub use skill::{Skill, SkillResult, SkillSeam};
pub use tool::{Tool, ToolResult, ToolSeam};

// Re-export the typed seam accessors for ergonomic use by plugins.
pub use llm::llm_adapter;
pub use prompt::prompt_registry;
pub use skill::skill_registry;
pub use tool::tool_registry;

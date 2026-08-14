//! Skill seam — the ctx.skills extension point.
//!
//! Each skill is a plugin that registers here with input/output schemas
//! and an execute function. Skills can be LLM-based, tool-based, or hybrid.

use async_trait::async_trait;
use pocker_core::error::Result;
use pocker_core::types::{SkillDefinition, SkillType};
use serde::{Deserialize, Serialize};

/// A skill plugin. Skills are higher-level capabilities built on top of
/// tools and/or LLM adapters.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Skill name (unique)
    fn name(&self) -> &str;

    /// Skill version
    fn version(&self) -> &str;

    /// Skill description
    fn description(&self) -> &str;

    /// Skill type (llm / tool / hybrid)
    fn skill_type(&self) -> SkillType;

    /// Input schema (JSON Schema)
    fn input_schema(&self) -> serde_json::Value;

    /// Output schema (JSON Schema)
    fn output_schema(&self) -> serde_json::Value;

    /// Required seams (e.g. ["ctx.llm"])
    fn requires(&self) -> Vec<String>;

    /// Execute the skill with the given input.
    async fn execute(&self, input: serde_json::Value) -> Result<SkillResult>;

    /// Convert to a SkillDefinition.
    fn definition(&self) -> SkillDefinition {
        SkillDefinition {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            skill_type: self.skill_type(),
            input_schema: self.input_schema(),
            output_schema: self.output_schema(),
            requires: self.requires(),
        }
    }
}

/// Result of a skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SkillResult {
    pub fn ok(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(message.into()),
        }
    }
}

/// Wrapper for the skill registry seam.
pub struct SkillSeam {
    pub name: String,
}

//! Skill seam — the ctx.skills extension point.
//!
//! Each skill is a plugin that registers here with input/output schemas
//! and an execute function. Skills can be LLM-based, tool-based, or hybrid.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use pocker_core::seam::SeamId;
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

    /// Required seams (e.g. `[`ctx.llm`]`)
    fn requires(&self) -> Vec<String>;

    /// Execute the skill with the given input.
    async fn execute(&self, input: serde_json::Value) -> Result<SkillResult>;

    /// Convert to a `SkillDefinition`.
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
    #[must_use]
    pub const fn ok(output: serde_json::Value) -> Self {
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
///
/// Stores the registered skills so the agent loop can discover and execute
/// them by name.
pub struct SkillSeam {
    pub name: String,
    skills: RwLock<HashMap<String, Arc<dyn Skill>>>,
}

impl SkillSeam {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            skills: RwLock::new(HashMap::new()),
        }
    }

    /// Register a skill implementation.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn register_skill(&self, skill: Arc<dyn Skill>) {
        self.skills
            .write()
            .unwrap()
            .insert(skill.name().to_string(), skill);
    }

    /// Look up a skill by name.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn get_skill(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.read().unwrap().get(name).cloned()
    }

    /// List all registered skill names.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn list_skills(&self) -> Vec<String> {
        self.skills.read().unwrap().keys().cloned().collect()
    }

    /// Number of registered skills.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn len(&self) -> usize {
        self.skills.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl pocker_core::seam::Seam for SkillSeam {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Helper: retrieve the mounted skill registry from a context, if any.
pub fn skill_registry(ctx: &Ctx) -> Option<Arc<SkillSeam>> {
    ctx.get_seam_typed::<SkillSeam>(&SeamId::skills())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillResult;
    use pocker_core::error::Result;
    use pocker_core::types::SkillType;

    struct DummySkill;

    #[async_trait]
    impl Skill for DummySkill {
        fn name(&self) -> &'static str {
            "summarize"
        }
        fn version(&self) -> &'static str {
            "1.0.0"
        }
        fn description(&self) -> &'static str {
            "summarizes text"
        }
        fn skill_type(&self) -> SkillType {
            SkillType::Llm
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object" })
        }
        fn output_schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "string" })
        }
        fn requires(&self) -> Vec<String> {
            vec!["ctx.llm".to_string()]
        }
        async fn execute(&self, _input: serde_json::Value) -> Result<SkillResult> {
            Ok(SkillResult::ok(serde_json::json!("done")))
        }
    }

    #[test]
    fn seam_registers_and_lists() {
        let seam = SkillSeam::new("skills");
        assert!(seam.is_empty());
        seam.register_skill(Arc::new(DummySkill));
        assert_eq!(seam.len(), 1);
        assert!(seam.get_skill("summarize").is_some());
        assert_eq!(seam.list_skills(), vec!["summarize".to_string()]);
        let def = DummySkill.definition();
        assert_eq!(def.requires, vec!["ctx.llm".to_string()]);
    }
}

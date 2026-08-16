//! Prompt/instruction seam — the ctx.prompt extension point.
//!
//! Holds Harness-style [`InstructionSkill`]s: Markdown instruction bodies that
//! an LLM router discovers and injects into context. This is deliberately
//! separate from the typed, executable [`crate::Skill`] trait — instruction
//! skills are prompt content, not functions with input/output schemas.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use pocker_core::context::Ctx;
use pocker_core::seam::SeamId;
use pocker_core::types::InstructionSkill;

/// Registry of instruction skills, mounted on `ctx.prompt`.
pub struct PromptSeam {
    name: String,
    skills: RwLock<HashMap<String, InstructionSkill>>,
}

impl PromptSeam {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            skills: RwLock::new(HashMap::new()),
        }
    }

    /// Register (or replace) an instruction skill by name.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn register(&self, skill: InstructionSkill) {
        self.skills
            .write()
            .unwrap()
            .insert(skill.name.clone(), skill);
    }

    /// Look up an instruction skill by name.
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn get(&self, name: &str) -> Option<InstructionSkill> {
        self.skills.read().unwrap().get(name).cloned()
    }

    /// List all registered instruction skills (cloned for safe return).
    ///
    /// # Panics
    /// Panics if the internal skill map lock is poisoned.
    pub fn list(&self) -> Vec<InstructionSkill> {
        self.skills.read().unwrap().values().cloned().collect()
    }

    /// Number of registered instruction skills.
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

impl pocker_core::seam::Seam for PromptSeam {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Helper: retrieve the mounted prompt/instruction registry from a context.
pub fn prompt_registry(ctx: &Ctx) -> Option<Arc<PromptSeam>> {
    ctx.get_seam_typed::<PromptSeam>(&SeamId::prompt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(name: &str) -> InstructionSkill {
        InstructionSkill {
            name: name.to_string(),
            description: "d".to_string(),
            when_to_use: None,
            content: "# instruction".to_string(),
            provider: "test".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn register_get_list() {
        let seam = PromptSeam::new("prompt");
        assert!(seam.is_empty());
        seam.register(skill("a"));
        seam.register(skill("b"));
        assert_eq!(seam.len(), 2);
        assert!(seam.get("a").is_some());
        assert!(seam.get("missing").is_none());
        let mut names = seam.list().into_iter().map(|s| s.name).collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn register_replaces_by_name() {
        let seam = PromptSeam::new("prompt");
        seam.register(skill("a"));
        let mut updated = skill("a");
        updated.content = "new".to_string();
        seam.register(updated);
        assert_eq!(seam.len(), 1);
        assert_eq!(seam.get("a").unwrap().content, "new");
    }
}

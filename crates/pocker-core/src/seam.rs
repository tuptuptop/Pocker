//! Seam — capability extension points.
//!
//! A seam is a named slot in the shared context where plugins register
//! their service implementations. Any part of Pocker is a seam:
//! `ctx.llm`, `ctx.tools`, `ctx.skills`, `ctx.sandbox`, etc.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A seam identifier (e.g. "ctx.llm", "ctx.tools").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeamId(pub String);

impl SeamId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Standard seam: LLM adapter
    #[must_use]
    pub fn llm() -> Self {
        Self::new("ctx.llm")
    }

    /// Standard seam: Tool registry
    #[must_use]
    pub fn tools() -> Self {
        Self::new("ctx.tools")
    }

    /// Standard seam: Skills registry
    #[must_use]
    pub fn skills() -> Self {
        Self::new("ctx.skills")
    }

    /// Standard seam: Instruction/prompt skill library (Harness-style
    /// Markdown instruction skills, discovered by an LLM router).
    #[must_use]
    pub fn prompt() -> Self {
        Self::new("ctx.prompt")
    }

    /// Standard seam: Session log
    #[must_use]
    pub fn session() -> Self {
        Self::new("ctx.session")
    }

    /// Standard seam: Sandbox
    #[must_use]
    pub fn sandbox() -> Self {
        Self::new("ctx.sandbox")
    }

    /// Standard seam: Approval
    #[must_use]
    pub fn approval() -> Self {
        Self::new("ctx.approval")
    }

    /// Standard seam: Filesystem
    #[must_use]
    pub fn fs() -> Self {
        Self::new("ctx.fs")
    }

    /// Standard seam: Terminal
    #[must_use]
    pub fn terminal() -> Self {
        Self::new("ctx.terminal")
    }

    /// Standard seam: Event bus
    #[must_use]
    pub fn bus() -> Self {
        Self::new("ctx.bus")
    }

    /// Standard seam: Credentials
    #[must_use]
    pub fn credentials() -> Self {
        Self::new("ctx.credentials")
    }
}

impl fmt::Display for SeamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A trait object stored in a seam.
///
/// `Seam` requires `Any` so that typed services can be retrieved back from the
/// registry via [`crate::context::Ctx::get_seam_typed`]. All concrete seam
/// wrappers are `'static`, so this bound is always satisfiable.
pub trait Seam: Send + Sync + std::any::Any {
    /// Name of this seam implementation.
    fn name(&self) -> &str;
}

/// A seam entry — a registered implementation.
pub(crate) struct SeamEntry {
    pub provider_name: String,
    pub implementation: Arc<dyn Seam>,
}

/// Registry of all seams.
pub struct SeamRegistry {
    entries: HashMap<SeamId, Vec<SeamEntry>>,
}

impl SeamRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register an implementation for a seam.
    pub fn register(&mut self, seam: SeamId, provider_name: String, implementation: Arc<dyn Seam>) {
        self.entries.entry(seam).or_default().push(SeamEntry {
            provider_name,
            implementation,
        });
    }

    /// Unregister all implementations from a specific provider.
    pub fn unregister_provider(&mut self, seam: &SeamId, provider_name: &str) {
        if let Some(entries) = self.entries.get_mut(seam) {
            entries.retain(|e| e.provider_name != provider_name);
        }
    }

    /// Get the first (default) implementation for a seam.
    #[must_use]
    pub fn get(&self, seam: &SeamId) -> Option<Arc<dyn Seam>> {
        self.entries
            .get(seam)
            .and_then(|entries| entries.first())
            .map(|e| e.implementation.clone())
    }

    /// List all registered seam names.
    #[must_use]
    pub fn list(&self) -> Vec<SeamId> {
        self.entries.keys().cloned().collect()
    }

    /// Check if a seam has any registered implementations.
    #[must_use]
    pub fn has(&self, seam: &SeamId) -> bool {
        self.entries.get(seam).is_some_and(|e| !e.is_empty())
    }
}

impl Default for SeamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummySeam {
        name: String,
    }

    impl Seam for DummySeam {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn seam_register_and_get() {
        let mut registry = SeamRegistry::new();
        let seam_id = SeamId::llm();
        let impl_arc = Arc::new(DummySeam {
            name: "openai".to_string(),
        }) as Arc<dyn Seam>;

        registry.register(seam_id.clone(), "openai-plugin".to_string(), impl_arc);

        assert!(registry.has(&seam_id));
        let got = registry.get(&seam_id).unwrap();
        assert_eq!(got.name(), "openai");
    }

    #[test]
    fn seam_unregister_provider() {
        let mut registry = SeamRegistry::new();
        let seam_id = SeamId::tools();

        registry.register(
            seam_id.clone(),
            "plugin-a".to_string(),
            Arc::new(DummySeam {
                name: "a".to_string(),
            }) as Arc<dyn Seam>,
        );
        registry.register(
            seam_id.clone(),
            "plugin-b".to_string(),
            Arc::new(DummySeam {
                name: "b".to_string(),
            }) as Arc<dyn Seam>,
        );

        assert!(registry.has(&seam_id));
        registry.unregister_provider(&seam_id, "plugin-a");
        assert!(registry.has(&seam_id));
        let got = registry.get(&seam_id).unwrap();
        assert_eq!(got.name(), "b");
    }

    #[test]
    fn seam_standard_ids() {
        assert_eq!(SeamId::llm().to_string(), "ctx.llm");
        assert_eq!(SeamId::tools().to_string(), "ctx.tools");
        assert_eq!(SeamId::skills().to_string(), "ctx.skills");
        assert_eq!(SeamId::sandbox().to_string(), "ctx.sandbox");
        assert_eq!(SeamId::prompt().to_string(), "ctx.prompt");
    }
}

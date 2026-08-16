//! Built-in plugins and the default bundle registry.
//!
//! The engine ships with a `@pocker/core-bundle` that pulls in the structural
//! seams (`tools`, `skills`, `llm`, `prompt`). Any profile that declares this
//! bundle therefore gets a working seam substrate to build on, without the
//! application having to register anything itself.

use crate::registry::PluginRegistry;
use async_trait::async_trait;
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use pocker_core::plugin::{Plugin, PluginMetadata};
use pocker_core::seam::SeamId;
use pocker_core::types::{PluginId, PluginType};
use pocker_plugin::llm::{LlmAdapter, LlmSeam};
use pocker_plugin::prompt::PromptSeam;
use pocker_plugin::skill::SkillSeam;
use pocker_plugin::tool::ToolSeam;
use std::sync::Arc;
use futures::StreamExt;

/// The built-in core plugin: provides the four structural seams.
pub struct CoreBundlePlugin {
    meta: PluginMetadata,
    tool_seam: Arc<ToolSeam>,
    skill_seam: Arc<SkillSeam>,
    llm_seam: Arc<LlmSeam>,
    prompt_seam: Arc<PromptSeam>,
}

impl CoreBundlePlugin {
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: {
                let mut m = PluginMetadata::new("@pocker/core", "1.0.0");
                m.description = "Core structural seams (tools, skills, llm, prompt)".into();
                m.plugin_type = PluginType::Bundle;
                m.provides = vec![
                    "ctx.tools".into(),
                    "ctx.skills".into(),
                    "ctx.llm".into(),
                    "ctx.prompt".into(),
                ];
                m
            },
            tool_seam: Arc::new(ToolSeam::new("tools")),
            skill_seam: Arc::new(SkillSeam::new("skills")),
            llm_seam: Arc::new(LlmSeam {
                adapter: Arc::new(NullLlmAdapter),
            }),
            prompt_seam: Arc::new(PromptSeam::new("prompt")),
        }
    }
}

impl Default for CoreBundlePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for CoreBundlePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        ctx.register_seam(
            SeamId::tools(),
            self.meta.name.clone(),
            self.tool_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::skills(),
            self.meta.name.clone(),
            self.skill_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::llm(),
            self.meta.name.clone(),
            self.llm_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::prompt(),
            self.meta.name.clone(),
            self.prompt_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        Ok(())
    }

    async fn unmount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        ctx.unregister_seam(&SeamId::tools(), &self.meta.name);
        ctx.unregister_seam(&SeamId::skills(), &self.meta.name);
        ctx.unregister_seam(&SeamId::llm(), &self.meta.name);
        ctx.unregister_seam(&SeamId::prompt(), &self.meta.name);
        Ok(())
    }
}

/// Placeholder LLM adapter used by the core bundle until a real adapter plugin
/// is mounted. It fails loudly (rather than silently returning empty output)
/// so misconfiguration surfaces immediately.
struct NullLlmAdapter;

#[async_trait]
impl LlmAdapter for NullLlmAdapter {
    fn name(&self) -> &str {
        "null"
    }

    fn stream(
        &self,
        _messages: Vec<pocker_core::types::Message>,
        _options: pocker_core::types::LlmOptions,
    ) -> Result<futures::stream::BoxStream<'static, Result<pocker_core::types::Chunk>>> {
        let chunks: Vec<Result<pocker_core::types::Chunk>> = vec![Ok(
            pocker_core::types::Chunk::Error {
                message: "no LLM adapter configured; mount an llm-adapter plugin".into(),
            },
        )];
        Ok(futures::stream::iter(chunks).boxed())
    }

    fn list_models(&self, _provider: &str) -> Result<Vec<pocker_core::types::ModelInfo>> {
        Ok(vec![])
    }

    fn capabilities(&self) -> pocker_core::types::LlmCapabilities {
        pocker_core::types::LlmCapabilities::default()
    }
}

/// Register the engine's built-in plugins and the default `@pocker/core-bundle`.
///
/// Called from every [`crate::runtime::Engine`] constructor so a freshly built
/// engine can immediately load the core bundle declared by default profiles.
pub fn register_defaults(plugins: &PluginRegistry, bundles: &crate::registry::BundleRegistry) {
    plugins.register(
        "@pocker/core",
        Arc::new(|| Arc::new(CoreBundlePlugin::new())),
    );
    bundles.register(
        "@pocker/core-bundle",
        vec![PluginId::new("@pocker/core", "1.0.0")],
    );
}

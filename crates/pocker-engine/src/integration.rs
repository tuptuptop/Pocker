//! End-to-end integration tests for the seam model.
//!
//! These tests mount real [`pocker_plugin`] seam implementations (Tool, Skill,
//! LLM, Prompt) into a [`Ctx`] through a [`Plugin`] and verify the full
//! lifecycle: registration, typed discovery, execution, streaming, and clean
//! unmount. They exercise the architecture the way a real plugin would, so a
//! regression in the seam/registry plumbing cannot pass silently.

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use pocker_core::plugin::{Plugin, PluginMetadata};
use pocker_core::seam::SeamId;
use pocker_core::types::{
    Chunk, InstructionSkill, LlmCapabilities, LlmOptions, Message, ModelInfo, PluginType,
    SkillDefinition, SkillType, ToolDefinition,
};
use pocker_plugin::llm::{LlmAdapter, LlmSeam};
use pocker_plugin::prompt::PromptSeam;
use pocker_plugin::skill::{Skill, SkillResult, SkillSeam};
use pocker_plugin::tool::{Tool, ToolResult, ToolSeam};
use std::sync::Arc;

/// A plugin that contributes one of every seam type, exercising the registry.
struct AllInOnePlugin {
    meta: PluginMetadata,
    tool_seam: Arc<ToolSeam>,
    skill_seam: Arc<SkillSeam>,
    llm_seam: Arc<LlmSeam>,
    prompt_seam: Arc<PromptSeam>,
}

impl AllInOnePlugin {
    fn new() -> Self {
        Self {
            meta: {
                let mut m = PluginMetadata::new("all-in-one", "1.0.0");
                m.plugin_type = PluginType::Bundle;
                m.provides = vec![
                    "ctx.tools".to_string(),
                    "ctx.skills".to_string(),
                    "ctx.llm".to_string(),
                    "ctx.prompt".to_string(),
                ];
                m
            },
            tool_seam: Arc::new(ToolSeam::new("tools")),
            skill_seam: Arc::new(SkillSeam::new("skills")),
            llm_seam: Arc::new(LlmSeam {
                adapter: Arc::new(EchoAdapter),
            }),
            prompt_seam: Arc::new(PromptSeam::new("prompt")),
        }
    }
}

#[async_trait]
impl Plugin for AllInOnePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    async fn mount(&self, ctx: &Arc<Ctx>) -> Result<()> {
        self.tool_seam
            .register_tool(Arc::new(UpperTool) as Arc<dyn Tool>);
        self.skill_seam
            .register_skill(Arc::new(ReverseSkill) as Arc<dyn Skill>);
        ctx.register_seam(
            SeamId::tools(),
            self.meta.name.clone(),
            self.meta.digest(),
            self.tool_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::skills(),
            self.meta.name.clone(),
            self.meta.digest(),
            self.skill_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::llm(),
            self.meta.name.clone(),
            self.meta.digest(),
            self.llm_seam.clone() as Arc<dyn pocker_core::seam::Seam>,
        );
        ctx.register_seam(
            SeamId::prompt(),
            self.meta.name.clone(),
            self.meta.digest(),
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

struct UpperTool;

#[async_trait]
impl Tool for UpperTool {
    fn name(&self) -> &str {
        "upper"
    }
    fn description(&self) -> &str {
        "uppercases input text"
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": { "text": { "type": "string" } } })
    }
    fn timeout_ms(&self) -> Option<u64> {
        Some(5000)
    }
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(ToolResult::ok(serde_json::json!(text.to_uppercase())))
    }
}

struct ReverseSkill;

#[async_trait]
impl Skill for ReverseSkill {
    fn name(&self) -> &str {
        "reverse"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "reverses a string"
    }
    fn skill_type(&self) -> SkillType {
        SkillType::Tool
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }
    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "string" })
    }
    fn requires(&self) -> Vec<String> {
        Vec::new()
    }
    async fn execute(&self, input: serde_json::Value) -> Result<SkillResult> {
        let s = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let reversed: String = s.chars().rev().collect();
        Ok(SkillResult::ok(serde_json::json!(reversed)))
    }
}

struct EchoAdapter;

#[async_trait]
impl LlmAdapter for EchoAdapter {
    fn name(&self) -> &str {
        "echo"
    }
    fn stream(
        &self,
        messages: Vec<Message>,
        _options: LlmOptions,
    ) -> Result<futures::stream::BoxStream<'static, Result<Chunk>>> {
        let last = messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let chunks: Vec<Result<Chunk>> = vec![
            Ok(Chunk::Text {
                content: format!("echo: {last}"),
            }),
            Ok(Chunk::Done { usage: None }),
        ];
        Ok(stream::iter(chunks).boxed())
    }
    fn list_models(&self, _provider: &str) -> Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: "echo-1".to_string(),
            name: "Echo 1".to_string(),
            context_window: 4096,
            provider: "echo".to_string(),
        }])
    }
    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities {
            function_calling: false,
            vision: false,
            streaming: true,
        }
    }
}

fn instruction(name: &str, content: &str) -> InstructionSkill {
    InstructionSkill {
        name: name.to_string(),
        description: "instruction".to_string(),
        when_to_use: None,
        content: content.to_string(),
        provider: "all-in-one".to_string(),
        version: "1.0.0".to_string(),
    }
}

#[tokio::test]
async fn end_to_end_seam_lifecycle() {
    let ctx = Arc::new(Ctx::new());
    let plugin = Arc::new(AllInOnePlugin::new());

    // Mount: every seam should appear in the context.
    plugin.mount(&ctx).await.unwrap();
    assert!(ctx.has_seam(&SeamId::tools()));
    assert!(ctx.has_seam(&SeamId::skills()));
    assert!(ctx.has_seam(&SeamId::llm()));
    assert!(ctx.has_seam(&SeamId::prompt()));

    // Typed discovery via the registry helpers.
    let tools = pocker_plugin::tool::tool_registry(&ctx).expect("tool registry");
    assert_eq!(tools.len(), 1);
    assert!(tools.get_tool("upper").is_some());

    let skills = pocker_plugin::skill::skill_registry(&ctx).expect("skill registry");
    assert_eq!(skills.len(), 1);
    assert!(skills.get_skill("reverse").is_some());

    let llm = pocker_plugin::llm::llm_adapter(&ctx).expect("llm adapter");
    assert_eq!(llm.name(), "echo");

    let prompt = pocker_plugin::prompt::prompt_registry(&ctx).expect("prompt registry");
    assert!(prompt.is_empty());

    // Execute the tool through the registry.
    let tool = tools.get_tool("upper").unwrap();
    let out = tool
        .execute(serde_json::json!({ "text": "hello" }))
        .await
        .unwrap();
    assert!(out.success);
    assert_eq!(out.output, serde_json::json!("HELLO"));

    // ToolDefinition carries the optional timeout.
    let def: ToolDefinition = tool.definition();
    assert_eq!(def.timeout_ms, Some(5000));

    // Execute the skill through the registry.
    let skill = skills.get_skill("reverse").unwrap();
    let sres = skill
        .execute(serde_json::json!({ "text": "abc" }))
        .await
        .unwrap();
    assert!(sres.success);
    assert_eq!(sres.output, serde_json::json!("cba"));

    let sdef: SkillDefinition = skill.definition();
    assert_eq!(sdef.skill_type, SkillType::Tool);

    // Stream from the LLM adapter; collect text chunks.
    let mut s = llm
        .stream(
            vec![Message {
                role: pocker_core::types::Role::User,
                content: "ping".to_string(),
                tool_call_id: None,
                tool_calls: None,
            }],
            LlmOptions::default(),
        )
        .unwrap();
    let mut collected = String::new();
    while let Some(item) = s.next().await {
        if let Chunk::Text { content } = item.unwrap() {
            collected.push_str(&content);
        }
    }
    assert_eq!(collected, "echo: ping");

    // Register and retrieve an instruction skill on the prompt seam.
    prompt.register(instruction("be-polite", "# Be polite"));
    let got = prompt.get("be-polite").expect("instruction present");
    assert_eq!(got.content, "# Be polite");
    assert_eq!(prompt.len(), 1);

    // Unmount: every seam should be removed cleanly.
    plugin.unmount(&ctx).await.unwrap();
    assert!(!ctx.has_seam(&SeamId::tools()));
    assert!(!ctx.has_seam(&SeamId::skills()));
    assert!(!ctx.has_seam(&SeamId::llm()));
    assert!(!ctx.has_seam(&SeamId::prompt()));
}

#[tokio::test]
async fn end_to_end_plugin_through_loader() {
    // Exercise the same flow through the real PluginLoader + Engine path.
    let engine = crate::Engine::new();
    let plugin = Arc::new(AllInOnePlugin::new());
    engine.register_plugin("all-in-one", plugin).unwrap();
    engine.mount_plugin("all-in-one").await.unwrap();
    assert!(engine.loader.is_mounted("all-in-one"));

    let ctx = engine.context();
    let tools = pocker_plugin::tool::tool_registry(ctx).expect("tool registry");
    let out = tools
        .get_tool("upper")
        .unwrap()
        .execute(serde_json::json!({ "text": "loader" }))
        .await
        .unwrap();
    assert_eq!(out.output, serde_json::json!("LOADER"));

    engine.unmount_plugin("all-in-one").await.unwrap();
    assert!(!engine.loader.is_mounted("all-in-one"));
    assert!(!ctx.has_seam(&SeamId::tools()));
}

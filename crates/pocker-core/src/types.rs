//! Pocker shared types.

use serde::{Deserialize, Serialize};

/// A message role in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A conversation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A tool/function call from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// A tool definition exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    /// Optional output schema (Harness-style). Lets the model and UI know the
    /// shape of a tool's result. `None` means unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Optional cooperative timeout budget in milliseconds (Harness-style).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// LLM streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Chunk {
    Text {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    Done {
        usage: Option<Usage>,
    },
    Error {
        message: String,
    },
}

/// Token usage info.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// LLM model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: u64,
    /// Owning provider (e.g. "openai", "ollama", "deepseek").
    /// Namespaced so multi-provider adapters can coexist without id collisions.
    #[serde(default)]
    pub provider: String,
}

/// LLM capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmCapabilities {
    pub function_calling: bool,
    pub vision: bool,
    pub streaming: bool,
}

/// Options for LLM generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmOptions {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub tools: Option<Vec<ToolDefinition>>,
}

/// Skill type classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillType {
    /// Requires LLM capability (calls ctx.llm)
    Llm,
    /// Pure tool-based (no LLM needed)
    Tool,
    /// Mixed: tool logic + LLM
    Hybrid,
}

/// Skill definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub version: String,
    pub description: String,
    pub skill_type: SkillType,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires: Vec<String>,
}

/// A Harness-style instruction skill.
///
/// This is a Markdown body discovered by an LLM router and injected into
/// context. Unlike [`SkillDefinition`] (a typed executable capability with
/// input/output schemas), this is prompt/instruction content that the model
/// "follows" rather than a function it calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSkill {
    /// Kebab-case identifier used to address the skill.
    pub name: String,
    /// Short routing description shown by discovery consumers.
    pub description: String,
    /// Optional extra routing guidance ("when to use this skill").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    /// Markdown instruction body.
    pub content: String,
    /// Provider that owns this skill body.
    #[serde(default)]
    pub provider: String,
    /// Skill version.
    #[serde(default)]
    pub version: String,
}

/// Plugin type classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// LLM model adapter (provides ctx.llm)
    LlmAdapter,
    /// Tool plugin (provides ctx.tools)
    Tool,
    /// Skill plugin (provides ctx.skills)
    Skill,
    /// Sandbox plugin (provides ctx.sandbox)
    Sandbox,
    /// Session log plugin (provides ctx.session)
    Session,
    /// Approval plugin (provides ctx.approval)
    Approval,
    /// UI plugin (CLI/TUI/Web)
    Ui,
    /// Bundle (collection of plugins)
    Bundle,
    /// Other
    Other,
}

/// Plugin identifier (name@version).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PluginId {
    pub name: String,
    pub version: String,
}

impl PluginId {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Self {
        // Use rsplit_once to handle scoped names like "@pocker/llm-openai@1.0.0"
        if let Some((name, version)) = s.rsplit_once('@') {
            // If the name is empty (e.g. input was just "@1.0.0"), treat the whole string as name
            if name.is_empty() {
                Self {
                    name: s.to_string(),
                    version: "0.0.0".to_string(),
                }
            } else {
                Self {
                    name: name.to_string(),
                    version: version.to_string(),
                }
            }
        } else {
            Self {
                name: s.to_string(),
                version: "0.0.0".to_string(),
            }
        }
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// A profile configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub bundles: Vec<String>,
    pub plugins: Vec<PluginId>,
    #[serde(default)]
    pub patch: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_id_parse_with_version() {
        let id = PluginId::parse("@pocker/llm-openai@1.0.0");
        assert_eq!(id.name, "@pocker/llm-openai");
        assert_eq!(id.version, "1.0.0");
    }

    #[test]
    fn plugin_id_parse_without_version() {
        let id = PluginId::parse("@pocker/llm-openai");
        assert_eq!(id.name, "@pocker/llm-openai");
        assert_eq!(id.version, "0.0.0");
    }

    #[test]
    fn plugin_id_display() {
        let id = PluginId::new("test", "1.0.0");
        assert_eq!(id.to_string(), "test@1.0.0");
    }

    #[test]
    fn message_serialization() {
        let msg = Message {
            role: Role::User,
            content: "Hello".to_string(),
            tool_call_id: None,
            tool_calls: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(back.role, Role::User);
        assert_eq!(back.content, "Hello");
    }

    #[test]
    fn skill_type_serialization() {
        let skill = SkillType::Llm;
        let json = serde_json::to_string(&skill).unwrap();
        assert_eq!(json, "\"llm\"");
    }
}

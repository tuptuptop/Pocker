//! Tool seam — the ctx.tools extension point.
//!
//! Tools are registered here and invoked by the agent loop or skills.

use async_trait::async_trait;
use pocker_core::error::Result;
use pocker_core::types::ToolDefinition;
use serde::{Deserialize, Serialize};

/// A tool that can be invoked by the LLM.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (unique within the registry)
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// Input schema (JSON Schema)
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the tool.
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;

    /// Convert to a ToolDefinition for LLM function calling.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
        }
    }
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
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

/// Wrapper to implement the Seam trait for the tool registry.
pub struct ToolSeam {
    pub name: String,
}

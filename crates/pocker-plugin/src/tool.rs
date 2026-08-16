//! Tool seam — the ctx.tools extension point.
//!
//! Tools are registered here and invoked by the agent loop or skills.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use pocker_core::context::Ctx;
use pocker_core::error::Result;
use pocker_core::seam::SeamId;
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

    /// Optional output schema (JSON Schema) for the tool's result.
    /// Defaults to `None` (unspecified) for backward compatibility.
    fn output_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Optional cooperative timeout budget in milliseconds.
    /// Defaults to `None` (no budget) for backward compatibility.
    fn timeout_ms(&self) -> Option<u64> {
        None
    }

    /// Execute the tool.
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;

    /// Convert to a `ToolDefinition` for LLM function calling.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
            output_schema: self.output_schema(),
            timeout_ms: self.timeout_ms(),
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

/// Wrapper to implement the Seam trait for the tool registry.
///
/// Unlike a bare name marker, this actually stores the registered tools so the
/// agent loop can discover and invoke them by name.
pub struct ToolSeam {
    pub name: String,
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolSeam {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool implementation.
    ///
    /// # Panics
    /// Panics if the internal tool map lock is poisoned.
    pub fn register_tool(&self, tool: Arc<dyn Tool>) {
        self.tools
            .write()
            .unwrap()
            .insert(tool.name().to_string(), tool);
    }

    /// Look up a tool by name.
    ///
    /// # Panics
    /// Panics if the internal tool map lock is poisoned.
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().unwrap().get(name).cloned()
    }

    /// List all registered tool names.
    ///
    /// # Panics
    /// Panics if the internal tool map lock is poisoned.
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.read().unwrap().keys().cloned().collect()
    }

    /// Number of registered tools.
    ///
    /// # Panics
    /// Panics if the internal tool map lock is poisoned.
    pub fn len(&self) -> usize {
        self.tools.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl pocker_core::seam::Seam for ToolSeam {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Helper: retrieve the mounted tool registry from a context, if any.
pub fn tool_registry(ctx: &Ctx) -> Option<Arc<ToolSeam>> {
    ctx.get_seam_typed::<ToolSeam>(&SeamId::tools())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocker_core::error::Result;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &'static str {
            "echo"
        }
        fn description(&self) -> &'static str {
            "echoes input"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object" })
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult> {
            Ok(ToolResult::ok(serde_json::json!("ok")))
        }
    }

    #[test]
    fn seam_registers_and_lists() {
        let seam = ToolSeam::new("tools");
        assert!(seam.is_empty());
        seam.register_tool(Arc::new(DummyTool));
        assert_eq!(seam.len(), 1);
        assert!(seam.get_tool("echo").is_some());
        assert!(seam.get_tool("missing").is_none());
        assert_eq!(seam.list_tools(), vec!["echo".to_string()]);
        // definition() carries the new optional fields.
        let def = DummyTool.definition();
        assert_eq!(def.name, "echo");
        assert!(def.output_schema.is_none());
        assert!(def.timeout_ms.is_none());
    }
}

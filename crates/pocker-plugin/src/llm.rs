//! LLM adapter seam — the ctx.llm extension point.
//!
//! LLM model adapters (OpenAI, Anthropic, Ollama, DeepSeek) register here.

use async_trait::async_trait;
use pocker_core::types::{Chunk, LlmCapabilities, LlmOptions, Message, ModelInfo};
use pocker_core::error::Result;
use std::sync::Arc;

/// The LLM seam trait. Plugins implement this to provide LLM access.
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Name of this adapter (e.g. "openai", "ollama")
    fn name(&self) -> &str;

    /// Stream a completion.
    async fn stream(
        &self,
        messages: Vec<Message>,
        options: LlmOptions,
    ) -> Result<Vec<Chunk>>;

    /// Generate a completion (non-streaming).
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: LlmOptions,
    ) -> Result<String>;

    /// List available models.
    fn models(&self) -> Vec<ModelInfo>;

    /// Adapter capabilities.
    fn capabilities(&self) -> LlmCapabilities;
}

/// Wrapper to implement the Seam trait for LlmAdapter.
pub struct LlmSeam {
    pub adapter: Arc<dyn LlmAdapter>,
}

impl pocker_core::seam::Seam for LlmSeam {
    fn name(&self) -> &str {
        self.adapter.name()
    }
}

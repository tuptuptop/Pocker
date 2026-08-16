//! LLM adapter seam — the ctx.llm extension point.
//!
//! LLM model adapters (`OpenAI`, Anthropic, Ollama, `DeepSeek`) register here.
//!
//! Streaming is real: [`LlmAdapter::stream`] returns a `BoxStream` that the
//! caller polls, so frontends can render tokens incrementally and long
//! generations never block. (Earlier versions returned `Vec<Chunk>`, which
//! buffered the entire response before yielding — see plugin-interface-
//! comparison.md.)

use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use pocker_core::error::Result;
use pocker_core::types::{Chunk, LlmCapabilities, LlmOptions, Message, ModelInfo};

/// The LLM seam trait. Plugins implement this to provide LLM access.
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Name of this adapter (e.g. "openai", "ollama"). Also used as the
    /// default provider namespace for its models.
    fn name(&self) -> &str;

    /// Stream a completion as a real async stream of [`Chunk`]s.
    ///
    /// The caller drives the stream; each `Text` chunk is a delta to render.
    /// Implementations should surface failures as `Chunk::Error` items or as
    /// an `Err` from this function (transport-level setup errors).
    ///
    /// # Errors
    /// Returns an error for transport/setup failures before streaming begins
    /// (e.g. missing API key, unreachable endpoint). Stream-time failures are
    /// emitted as [`Chunk::Error`] items, not as an `Err` here.
    fn stream(
        &self,
        messages: Vec<Message>,
        options: LlmOptions,
    ) -> Result<BoxStream<'static, Result<Chunk>>>;

    /// Generate a complete (non-streaming) completion.
    ///
    /// Default implementation collects [`LlmAdapter::stream`]; adapters may
    /// override for a more efficient path. `ToolCall` and `Done` chunks are
    /// ignored; a `Chunk::Error` aborts with [`pocker_core::error::PockerError::Runtime`].
    async fn generate(&self, messages: Vec<Message>, options: LlmOptions) -> Result<String> {
        let mut stream = self.stream(messages, options)?;
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            match item? {
                Chunk::Text { content } => out.push_str(&content),
                Chunk::ToolCall { .. } | Chunk::Done { .. } => {}
                Chunk::Error { message } => {
                    return Err(pocker_core::error::PockerError::Runtime(message));
                }
            }
        }
        Ok(out)
    }

    /// List available models scoped to a provider.
    ///
    /// Multi-provider aware: `provider` selects the namespace (e.g. "openai").
    /// Single-provider adapters may ignore it or use `self.name()`.
    ///
    /// # Errors
    /// Returns an error if the provider cannot be reached or its model list
    /// cannot be enumerated.
    fn list_models(&self, provider: &str) -> Result<Vec<ModelInfo>>;

    /// Adapter capabilities.
    fn capabilities(&self) -> LlmCapabilities;
}

/// Wrapper to implement the Seam trait for `LlmAdapter`.
pub struct LlmSeam {
    pub adapter: Arc<dyn LlmAdapter>,
}

impl pocker_core::seam::Seam for LlmSeam {
    fn name(&self) -> &str {
        self.adapter.name()
    }
}

/// Helper: retrieve the mounted LLM adapter from a context, if any.
pub fn llm_adapter(ctx: &pocker_core::context::Ctx) -> Option<Arc<dyn LlmAdapter>> {
    ctx.get_seam_typed::<LlmSeam>(&pocker_core::seam::SeamId::llm())
        .map(|s| s.adapter.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::StreamExt;
    use pocker_core::types::{Chunk, LlmCapabilities, LlmOptions, Message, ModelInfo};

    struct MockAdapter;

    #[async_trait]
    impl LlmAdapter for MockAdapter {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn stream(
            &self,
            _messages: Vec<Message>,
            _options: LlmOptions,
        ) -> Result<BoxStream<'static, Result<Chunk>>> {
            let chunks: Vec<Result<Chunk>> = vec![
                Ok(Chunk::Text {
                    content: "Hello ".into(),
                }),
                Ok(Chunk::Text {
                    content: "world".into(),
                }),
                Ok(Chunk::Done { usage: None }),
            ];
            Ok(stream::iter(chunks).boxed())
        }
        fn list_models(&self, _provider: &str) -> Result<Vec<ModelInfo>> {
            Ok(vec![ModelInfo {
                id: "m1".into(),
                name: "M1".into(),
                context_window: 8192,
                provider: "mock".into(),
            }])
        }
        fn capabilities(&self) -> LlmCapabilities {
            LlmCapabilities {
                function_calling: true,
                vision: false,
                streaming: true,
            }
        }
    }

    struct ErrAdapter;

    #[async_trait]
    impl LlmAdapter for ErrAdapter {
        fn name(&self) -> &'static str {
            "err"
        }
        fn stream(
            &self,
            _messages: Vec<Message>,
            _options: LlmOptions,
        ) -> Result<BoxStream<'static, Result<Chunk>>> {
            Ok(stream::iter(vec![Ok(Chunk::Error {
                message: "boom".into(),
            })])
            .boxed())
        }
        fn list_models(&self, _provider: &str) -> Result<Vec<ModelInfo>> {
            Ok(vec![])
        }
        fn capabilities(&self) -> LlmCapabilities {
            LlmCapabilities::default()
        }
    }

    #[tokio::test]
    async fn generate_collects_stream() {
        let out = MockAdapter
            .generate(vec![], LlmOptions::default())
            .await
            .unwrap();
        assert_eq!(out, "Hello world");
    }

    #[tokio::test]
    async fn stream_yields_individual_chunks() {
        let mut s = MockAdapter.stream(vec![], LlmOptions::default()).unwrap();
        let mut collected = String::new();
        while let Some(item) = s.next().await {
            if let Chunk::Text { content } = item.unwrap() {
                collected.push_str(&content);
            }
        }
        assert_eq!(collected, "Hello world");
    }

    #[tokio::test]
    async fn generate_propagates_error_chunk() {
        let r = ErrAdapter.generate(vec![], LlmOptions::default()).await;
        assert!(r.is_err());
    }
}

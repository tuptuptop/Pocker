//! Runtime configuration for the Pocker agent backend.
//!
//! Replaces the Python Hermes agent's env contract (`HERMES_HOME/.env`).
//! All values are read from the process environment with sensible defaults
//! so the binary can run with zero configuration against OpenAI-compatible
//! providers.

use std::env;

/// Which upstream LLM provider the backend proxies to.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Provider {
    #[default]
    OpenAi,
    OpenRouter,
    Anthropic,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::OpenAi => "openai",
            Provider::OpenRouter => "openrouter",
            Provider::Anthropic => "anthropic",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub host: String,
    pub port: u16,
    /// Upstream base URL, e.g. https://api.openai.com/v1
    pub base_url: String,
    pub api_key: String,
    /// Default model used when the client does not specify one.
    pub model: String,
    pub provider: Provider,
}

impl Config {
    pub fn from_env() -> Self {
        let provider = match env::var("POCKER_AGENT_PROVIDER")
            .unwrap_or_else(|_| "openai".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "openrouter" => Provider::OpenRouter,
            "anthropic" => Provider::Anthropic,
            _ => Provider::OpenAi,
        };

        let (base_url, api_key, model) = match provider {
            Provider::OpenRouter => (
                env::var("OPENROUTER_BASE_URL")
                    .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into()),
                env::var("OPENROUTER_API_KEY").unwrap_or_default(),
                env::var("POCKER_AGENT_MODEL").unwrap_or_else(|_| "openai/gpt-4o-mini".into()),
            ),
            Provider::Anthropic => (
                "https://api.anthropic.com".into(),
                env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
                env::var("POCKER_AGENT_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
            ),
            Provider::OpenAi => (
                env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into()),
                env::var("OPENAI_API_KEY").unwrap_or_default(),
                env::var("POCKER_AGENT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            ),
        };

        let host = env::var("POCKER_AGENT_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("POCKER_AGENT_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8642);

        Config {
            host,
            port,
            base_url,
            api_key,
            model,
            provider,
        }
    }
}

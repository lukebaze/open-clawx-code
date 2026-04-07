//! Multi-provider abstraction for LLM backends.
//!
//! Provides a unified `Provider` trait with adapters for Anthropic, `OpenAI`,
//! Gemini, Ollama, and Groq.

mod anthropic;
mod gemini;
mod groq;
mod ollama;
mod openai;
pub mod registry;

use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use registry::ProviderRegistry;

/// Metadata about a model offered by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u32,
    pub max_output: u32,
    pub supports_tools: bool,
    pub supports_vision: bool,
    /// Cost per million input tokens (USD).
    pub input_cost_per_m: f64,
    /// Cost per million output tokens (USD).
    pub output_cost_per_m: f64,
}

/// A streaming chunk from the provider.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Incremental text delta.
    TextDelta(String),
    /// Tool use request from the model.
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    /// Usage stats for the turn.
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    /// End of message.
    Done,
}

/// Boxed async stream of chunks.
pub type MessageStream = Pin<Box<dyn futures_like::Stream<Item = StreamChunk> + Send>>;

/// A request to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub system: Vec<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Unified provider trait for all LLM backends.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai").
    fn name(&self) -> &'static str;

    /// List available models.
    fn models(&self) -> Vec<ModelInfo>;

    /// Send a message and get a streaming response.
    /// Returns chunks as a `Vec` (simplified; real streaming in Phase 08+).
    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>>;

    /// Validate that the API key is working.
    async fn validate_key(&self) -> anyhow::Result<bool>;

    /// Estimate cost in USD for given token counts.
    fn estimate_cost(&self, model: &str, input_tokens: u32, output_tokens: u32) -> f64;
}

/// Placeholder module — real `futures::Stream` not needed yet.
/// Using `Vec<StreamChunk>` return in `send_message` for simplicity.
mod futures_like {
    pub trait Stream {
        type Item;
    }
}

/// Auto-detect available providers from environment variables.
#[must_use]
pub fn detect_providers() -> Vec<Box<dyn Provider>> {
    let mut providers: Vec<Box<dyn Provider>> = Vec::new();

    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            providers.push(Box::new(AnthropicProvider::new(key)));
        }
    }

    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            providers.push(Box::new(OpenAiProvider::new(key)));
        }
    }

    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.is_empty() {
            providers.push(Box::new(GeminiProvider::new(key)));
        }
    }

    if let Ok(key) = std::env::var("GROQ_API_KEY") {
        if !key.is_empty() {
            providers.push(Box::new(GroqProvider::new(key)));
        }
    }

    // Ollama doesn't need an API key — check if running
    providers.push(Box::new(OllamaProvider::default()));

    providers
}

use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

/// Anthropic provider (Claude models).
pub struct AnthropicProvider {
    api_key: String,
}

impl AnthropicProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                provider: "anthropic".into(),
                context_window: 200_000,
                max_output: 16_384,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 3.0,
                output_cost_per_m: 15.0,
            },
            ModelInfo {
                id: "claude-opus-4-20250514".into(),
                name: "Claude Opus 4".into(),
                provider: "anthropic".into(),
                context_window: 200_000,
                max_output: 32_000,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 15.0,
                output_cost_per_m: 75.0,
            },
            ModelInfo {
                id: "claude-haiku-4-20250514".into(),
                name: "Claude Haiku 4".into(),
                provider: "anthropic".into(),
                context_window: 200_000,
                max_output: 8_192,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 0.8,
                output_cost_per_m: 4.0,
            },
        ]
    }

    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        // Stub — real integration wraps claw-api AnthropicClient
        let _ = &self.api_key;
        Ok(vec![
            StreamChunk::TextDelta(format!(
                "[anthropic/{}] provider connected but streaming not yet wired",
                request.model
            )),
            StreamChunk::Usage {
                input_tokens: 0,
                output_tokens: 0,
            },
            StreamChunk::Done,
        ])
    }

    async fn validate_key(&self) -> anyhow::Result<bool> {
        Ok(!self.api_key.is_empty())
    }

    fn estimate_cost(&self, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        let (input_rate, output_rate) = match model {
            m if m.contains("opus") => (15.0, 75.0),
            m if m.contains("haiku") => (0.8, 4.0),
            _ => (3.0, 15.0), // sonnet default
        };
        (f64::from(input_tokens) * input_rate + f64::from(output_tokens) * output_rate)
            / 1_000_000.0
    }
}

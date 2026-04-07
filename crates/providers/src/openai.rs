use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

/// `OpenAI` provider (GPT models).
pub struct OpenAiProvider {
    api_key: String,
}

impl OpenAiProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                provider: "openai".into(),
                context_window: 128_000,
                max_output: 16_384,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 2.5,
                output_cost_per_m: 10.0,
            },
            ModelInfo {
                id: "gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                provider: "openai".into(),
                context_window: 128_000,
                max_output: 16_384,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 0.15,
                output_cost_per_m: 0.6,
            },
            ModelInfo {
                id: "o3".into(),
                name: "o3".into(),
                provider: "openai".into(),
                context_window: 200_000,
                max_output: 100_000,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 10.0,
                output_cost_per_m: 40.0,
            },
        ]
    }

    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let _ = &self.api_key;
        Ok(vec![
            StreamChunk::TextDelta(format!(
                "[openai/{}] provider connected but streaming not yet wired",
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
            "gpt-4o-mini" => (0.15, 0.6),
            "o3" => (10.0, 40.0),
            _ => (2.5, 10.0),
        };
        (f64::from(input_tokens) * input_rate + f64::from(output_tokens) * output_rate)
            / 1_000_000.0
    }
}

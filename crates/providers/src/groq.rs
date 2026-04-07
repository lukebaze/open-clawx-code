use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

/// Groq provider — uses OpenAI-compatible API with Groq endpoint.
pub struct GroqProvider {
    api_key: String,
}

impl GroqProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn name(&self) -> &'static str {
        "groq"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "llama-3.3-70b-versatile".into(),
                name: "Llama 3.3 70B (Groq)".into(),
                provider: "groq".into(),
                context_window: 128_000,
                max_output: 32_768,
                supports_tools: true,
                supports_vision: false,
                input_cost_per_m: 0.59,
                output_cost_per_m: 0.79,
            },
            ModelInfo {
                id: "deepseek-r1-distill-llama-70b".into(),
                name: "DeepSeek R1 70B (Groq)".into(),
                provider: "groq".into(),
                context_window: 128_000,
                max_output: 16_384,
                supports_tools: false,
                supports_vision: false,
                input_cost_per_m: 0.75,
                output_cost_per_m: 0.99,
            },
        ]
    }

    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let _ = &self.api_key;
        Ok(vec![
            StreamChunk::TextDelta(format!(
                "[groq/{}] provider connected but streaming not yet wired",
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

    fn estimate_cost(&self, _model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        // Average Groq pricing
        (f64::from(input_tokens) * 0.59 + f64::from(output_tokens) * 0.79) / 1_000_000.0
    }
}

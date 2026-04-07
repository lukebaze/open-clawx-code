use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

/// Google Gemini provider.
pub struct GeminiProvider {
    api_key: String,
}

impl GeminiProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gemini-2.5-pro".into(),
                name: "Gemini 2.5 Pro".into(),
                provider: "gemini".into(),
                context_window: 1_000_000,
                max_output: 65_536,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 1.25,
                output_cost_per_m: 10.0,
            },
            ModelInfo {
                id: "gemini-2.5-flash".into(),
                name: "Gemini 2.5 Flash".into(),
                provider: "gemini".into(),
                context_window: 1_000_000,
                max_output: 65_536,
                supports_tools: true,
                supports_vision: true,
                input_cost_per_m: 0.15,
                output_cost_per_m: 0.6,
            },
        ]
    }

    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let _ = &self.api_key;
        Ok(vec![
            StreamChunk::TextDelta(format!(
                "[gemini/{}] provider connected but streaming not yet wired",
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
        let (input_rate, output_rate) = if model.contains("flash") {
            (0.15, 0.6)
        } else {
            (1.25, 10.0)
        };
        (f64::from(input_tokens) * input_rate + f64::from(output_tokens) * output_rate)
            / 1_000_000.0
    }
}

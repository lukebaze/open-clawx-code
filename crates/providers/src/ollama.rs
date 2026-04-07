use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Ollama provider for local models — no API key required.
pub struct OllamaProvider {
    base_url: String,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_OLLAMA_URL.to_string(),
        }
    }
}

impl OllamaProvider {
    #[must_use]
    pub fn with_url(url: String) -> Self {
        Self { base_url: url }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn models(&self) -> Vec<ModelInfo> {
        // Static list — real impl would query /api/tags
        vec![ModelInfo {
            id: "ollama/llama3.3".into(),
            name: "Llama 3.3 (local)".into(),
            provider: "ollama".into(),
            context_window: 128_000,
            max_output: 4_096,
            supports_tools: false,
            supports_vision: false,
            input_cost_per_m: 0.0,
            output_cost_per_m: 0.0,
        }]
    }

    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let _ = &self.base_url;
        Ok(vec![
            StreamChunk::TextDelta(format!(
                "[ollama/{}] provider connected but streaming not yet wired",
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
        // Check if Ollama is running
        let url = format!("{}/api/tags", self.base_url);
        let resp = reqwest::get(&url).await;
        Ok(resp.is_ok())
    }

    fn estimate_cost(&self, _model: &str, _input_tokens: u32, _output_tokens: u32) -> f64 {
        0.0 // local models are free
    }
}

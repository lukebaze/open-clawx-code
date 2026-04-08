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

    #[allow(clippy::cast_possible_truncation)]
    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let client = reqwest::Client::new();

        // Build messages: optional system prompt followed by conversation messages
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if !request.system.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system.join("\n")
            }));
        }
        for m in &request.messages {
            messages.push(serde_json::json!({
                "role": m.role,
                "content": m.content
            }));
        }

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": false
        });

        let url = format!("{}/api/chat", self.base_url);
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error ({status}): {error_body}");
        }

        let json: serde_json::Value = response.json().await?;

        let text = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Ollama returns token counts in eval_count / prompt_eval_count
        let input_tokens = json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["eval_count"].as_u64().unwrap_or(0) as u32;

        Ok(vec![
            StreamChunk::TextDelta(text),
            StreamChunk::Usage {
                input_tokens,
                output_tokens,
            },
            StreamChunk::Done,
        ])
    }

    async fn validate_key(&self) -> anyhow::Result<bool> {
        // No auth — just check if Ollama is reachable
        let url = format!("{}/api/tags", self.base_url);
        let resp = reqwest::get(&url).await;
        Ok(resp.is_ok())
    }

    fn estimate_cost(&self, _model: &str, _input_tokens: u32, _output_tokens: u32) -> f64 {
        0.0 // local models are free
    }
}

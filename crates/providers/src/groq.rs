use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Groq provider — uses OpenAI-compatible API at the Groq endpoint.
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
            "max_tokens": request.max_tokens
        });

        let url = format!("{GROQ_BASE_URL}/chat/completions");
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Groq API error ({status}): {error_body}");
        }

        let json: serde_json::Value = response.json().await?;

        let text = json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

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
        if self.api_key.is_empty() {
            return Ok(false);
        }
        let client = reqwest::Client::new();
        let url = format!("{GROQ_BASE_URL}/models");
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(resp.status() != reqwest::StatusCode::UNAUTHORIZED)
    }

    fn estimate_cost(&self, _model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        // Average Groq pricing
        (f64::from(input_tokens) * 0.59 + f64::from(output_tokens) * 0.79) / 1_000_000.0
    }
}

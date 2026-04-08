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

    #[allow(clippy::cast_possible_truncation)]
    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let client = reqwest::Client::new();

        // Build messages array from the request
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        // Concatenate system prompts with newlines
        let system = request.system.join("\n");

        let body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_tokens,
            "system": system,
            "messages": messages
        });

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({status}): {error_body}");
        }

        let json: serde_json::Value = response.json().await?;

        let text = json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens = json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

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
        // Quick validation: send a minimal request; 401 means invalid key
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-haiku-4-20250514",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await?;
        // 200 or 400 (bad request body) means key authenticated; 401 means invalid
        Ok(resp.status() != reqwest::StatusCode::UNAUTHORIZED)
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

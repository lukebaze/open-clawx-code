use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

/// `OpenAI` provider — also serves as a base for OpenAI-compatible APIs.
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    provider_name: &'static str,
}

impl OpenAiProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            provider_name: "openai",
        }
    }

    /// Create with a custom base URL for any OpenAI-compatible endpoint
    /// (Groq, `DeepSeek`, Together, local vLLM, etc.).
    #[must_use]
    pub fn with_base_url(api_key: String, base_url: String, provider_name: &'static str) -> Self {
        Self {
            api_key,
            base_url,
            provider_name,
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str {
        self.provider_name
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

        let url = format!("{}/chat/completions", self.base_url);
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
            anyhow::bail!("OpenAI API error ({status}): {error_body}");
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
        let url = format!("{}/models", self.base_url);
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(resp.status() != reqwest::StatusCode::UNAUTHORIZED)
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

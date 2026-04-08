use async_trait::async_trait;

use crate::{MessageRequest, ModelInfo, Provider, StreamChunk};

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

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

    #[allow(clippy::cast_possible_truncation)]
    async fn send_message(&self, request: &MessageRequest) -> anyhow::Result<Vec<StreamChunk>> {
        let client = reqwest::Client::new();

        // Build contents array from conversation messages.
        // System prompt is prepended as a user turn if present.
        let mut contents: Vec<serde_json::Value> = Vec::new();
        if !request.system.is_empty() {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": request.system.join("\n") }]
            }));
            // Gemini requires alternating turns — add a brief model ack
            contents.push(serde_json::json!({
                "role": "model",
                "parts": [{ "text": "Understood." }]
            }));
        }
        for m in &request.messages {
            // Gemini uses "model" instead of "assistant"
            let role = if m.role == "assistant" { "model" } else { &m.role };
            contents.push(serde_json::json!({
                "role": role,
                "parts": [{ "text": m.content }]
            }));
        }

        let body = serde_json::json!({ "contents": contents });

        // URL format: /v1beta/models/{model}:generateContent?key={api_key}
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_BASE_URL, request.model, self.api_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({status}): {error_body}");
        }

        let json: serde_json::Value = response.json().await?;

        let text = json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p["text"].as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens =
            json["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let output_tokens =
            json["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;

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
        // List models endpoint to check key validity
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            self.api_key
        );
        let resp = client.get(&url).send().await?;
        Ok(resp.status() != reqwest::StatusCode::UNAUTHORIZED
            && resp.status() != reqwest::StatusCode::FORBIDDEN)
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

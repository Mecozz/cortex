use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, Provider, ProviderError};

pub struct ClaudeProvider {
    client: Client,
    api_key: String,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
    model: String,
}

#[derive(Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        if !self.is_available() {
            return Err(ProviderError::NotConfigured("api_key_anthropic".into()));
        }

        let messages: Vec<AnthropicMessage> = request
            .messages
            .into_iter()
            .map(|m| AnthropicMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let body = AnthropicRequest {
            model: request.model,
            max_tokens: request.max_tokens,
            system: request.system_prompt,
            messages,
        };

        let req = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        let req = if self.api_key.starts_with("sk-ant-oat01-") {
            req.header("Authorization", format!("Bearer {}", self.api_key))
        } else {
            req.header("x-api-key", &self.api_key)
        };
        let resp = req
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<AnthropicError>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(ProviderError::ApiError { status, body: msg });
        }

        let ar: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        let content = ar
            .content
            .into_iter()
            .filter(|c| c.content_type == "text")
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content,
            input_tokens: ar.usage.input_tokens,
            output_tokens: ar.usage.output_tokens,
            model: ar.model,
            provider: "claude".into(),
        })
    }
}

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

fn fresh_oauth_token() -> Option<String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let path = std::path::Path::new(&home)
        .join(".claude")
        .join(".credentials.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v["claudeAiOauth"]["accessToken"]
        .as_str()
        .map(|s| s.to_string())
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

fn build_request(
    client: &Client,
    token: &str,
    body: &AnthropicRequest,
) -> reqwest::RequestBuilder {
    let req = client
        .post("https://api.anthropic.com/v1/messages")
        .header("anthropic-version", "2023-06-01")
        .json(body);
    if token.starts_with("sk-ant-oat01-") {
        req.header("Authorization", format!("Bearer {}", token))
    } else {
        req.header("x-api-key", token)
    }
}

async fn parse_response(
    resp: reqwest::Response,
) -> Result<CompletionResponse, ProviderError> {
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

        let resp = build_request(&self.client, &self.api_key, &body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        // On 401 with OAuth token, refresh from credentials file and retry once
        if resp.status().as_u16() == 401 && self.api_key.starts_with("sk-ant-oat01-") {
            if let Some(fresh) = fresh_oauth_token() {
                let resp2 = build_request(&self.client, &fresh, &body)
                    .send()
                    .await
                    .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
                return parse_response(resp2).await;
            }
        }

        parse_response(resp).await
    }
}

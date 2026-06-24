use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, Provider, ProviderError};

const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_TOKEN_URL: &str = "https://console.anthropic.com/api/oauth/token";

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

fn credentials_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    Some(
        std::path::Path::new(&home)
            .join(".claude")
            .join(".credentials.json"),
    )
}

fn read_access_token() -> Option<String> {
    let content = std::fs::read_to_string(credentials_path()?).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v["claudeAiOauth"]["accessToken"]
        .as_str()
        .map(|s| s.to_string())
}

async fn do_token_refresh(client: &Client) -> Option<String> {
    let path = credentials_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let creds: serde_json::Value = serde_json::from_str(&content).ok()?;
    let refresh_token = creds["claudeAiOauth"]["refreshToken"].as_str()?;

    let resp = client
        .post(OAUTH_TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": OAUTH_CLIENT_ID
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let new_token = body["access_token"].as_str()?.to_string();

    // Write refreshed token back so Claude Code stays in sync
    let mut updated = creds.clone();
    updated["claudeAiOauth"]["accessToken"] = serde_json::Value::String(new_token.clone());
    if let Some(exp) = body["expires_in"].as_u64() {
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as u64
            + exp * 1000;
        updated["claudeAiOauth"]["expiresAt"] =
            serde_json::Value::Number(expires_at.into());
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&updated).ok()?);

    Some(new_token)
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

async fn parse_response(resp: reqwest::Response) -> Result<CompletionResponse, ProviderError> {
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

        // For OAuth tokens, always read the freshest token from disk first
        let token = if self.api_key.starts_with("sk-ant-oat01-") {
            read_access_token().unwrap_or_else(|| self.api_key.clone())
        } else {
            self.api_key.clone()
        };

        let resp = build_request(&self.client, &token, &body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        // On 401 with OAuth, do a real refresh via the OAuth endpoint and retry
        if resp.status().as_u16() == 401 && self.api_key.starts_with("sk-ant-oat01-") {
            if let Some(fresh) = do_token_refresh(&self.client).await {
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

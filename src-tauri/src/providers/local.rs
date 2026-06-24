use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{CompletionRequest, CompletionResponse, Message, Provider, ProviderError};

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub fn default() -> Self {
        Self::new("http://localhost:11434".into())
    }
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

impl From<Message> for OllamaMessage {
    fn from(m: Message) -> Self {
        OllamaMessage {
            role: m.role,
            content: m.content,
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        let mut messages: Vec<OllamaMessage> =
            request.messages.into_iter().map(OllamaMessage::from).collect();

        if let Some(system) = request.system_prompt {
            messages.insert(
                0,
                OllamaMessage {
                    role: "system".into(),
                    content: system,
                },
            );
        }

        let body = OllamaRequest {
            model: request.model.clone(),
            messages,
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError { status, body });
        }

        let or: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(e.to_string()))?;

        Ok(CompletionResponse {
            content: or.message.content,
            input_tokens: or.prompt_eval_count.unwrap_or(0),
            output_tokens: or.eval_count.unwrap_or(0),
            model: request.model,
            provider: "ollama".into(),
        })
    }
}

use reqwest::Client;
use serde::Deserialize;

use crate::core::types::Message;

pub struct ExtractedFact {
    pub content: String,
    pub category: String,
    pub confidence: f32,
}

#[derive(Deserialize)]
struct RawFact {
    content: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
}

fn default_category() -> String {
    "other".into()
}
fn default_confidence() -> f32 {
    0.7
}

const EXTRACT_SYSTEM: &str = "Extract personal facts the user has stated about themselves. Focus on identity (name, age, location, job, family), preferences (likes, dislikes, habits), and explicit declarations (\"remember that...\", \"I want you to know...\"). Return ONLY a JSON array: [{\"content\": \"User's name is Alice\", \"category\": \"identity|preference|declaration|other\", \"confidence\": 0.95}]. Return [] if nothing memorable. No other text.";

/// INSTCAP — extract memorable facts from a conversation snippet using Claude Haiku.
pub async fn extract(messages: &[Message], api_key: &str) -> Vec<ExtractedFact> {
    if api_key.is_empty() || messages.is_empty() {
        return vec![];
    }

    let snippet = messages
        .iter()
        .rev()
        .take(6)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 512,
        "system": EXTRACT_SYSTEM,
        "messages": [{"role": "user", "content": snippet}]
    });

    let resp = match Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    if !resp.status().is_success() {
        return vec![];
    }

    #[derive(Deserialize)]
    struct ContentBlock {
        text: String,
    }
    #[derive(Deserialize)]
    struct ApiResp {
        content: Vec<ContentBlock>,
    }

    let parsed: ApiResp = match resp.json().await {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    let text = parsed.content.into_iter().map(|c| c.text).collect::<String>();
    let raw: Vec<RawFact> = serde_json::from_str(&text).unwrap_or_default();

    raw.into_iter()
        .filter(|f| !f.content.is_empty())
        .map(|f| ExtractedFact {
            content: f.content,
            category: f.category,
            confidence: f.confidence.clamp(0.0, 1.0),
        })
        .collect()
}

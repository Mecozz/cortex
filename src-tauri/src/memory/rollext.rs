use crate::core::types::Message;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
struct ScoredItem {
    content: String,
    score: u8,
}

const ROLLEXT_SYSTEM: &str = "You are a memory scoring engine. \
Score each unique piece of information in the conversation for long-term memorability (1-10). \
Return a JSON array: [{\"content\": \"brief fact\", \"score\": N}]. \
Only include information worth remembering. Omit greetings and filler. Return [] if nothing notable.";

pub async fn score(messages: &[Message], api_key: &str) -> Vec<(String, u8)> {
    if api_key.is_empty() {
        return vec![];
    }
    let history = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "system": ROLLEXT_SYSTEM,
        "messages": [{"role": "user", "content": history}]
    });

    let client = reqwest::Client::new();
    let resp = match client
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
    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return vec![],
    };
    let text = json["content"][0]["text"].as_str().unwrap_or("");
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let items: Vec<ScoredItem> = serde_json::from_str(cleaned).unwrap_or_default();
    items.into_iter().map(|i| (i.content, i.score)).collect()
}

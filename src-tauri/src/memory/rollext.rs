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

/// ROLLEXT — score conversation info for long-term memory. Uses the Anthropic
/// API if an API key is set, else the Claude Code subscription.
pub async fn score(messages: &[Message], api_key: &str) -> Vec<(String, u8)> {
    if messages.is_empty() {
        return vec![];
    }
    let history = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let text = match crate::core::llm::haiku(api_key, ROLLEXT_SYSTEM, &history, 1024).await {
        Some(t) => t,
        None => return vec![],
    };
    let items: Vec<ScoredItem> =
        serde_json::from_str(crate::core::llm::json_slice(&text)).unwrap_or_default();
    items.into_iter().map(|i| (i.content, i.score)).collect()
}

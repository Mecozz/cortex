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

/// INSTCAP — extract memorable facts from a conversation snippet. Uses the
/// Anthropic API if an API key is set, else the Claude Code subscription.
pub async fn extract(messages: &[Message], api_key: &str) -> Vec<ExtractedFact> {
    if messages.is_empty() {
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

    let text = match crate::core::llm::haiku(api_key, EXTRACT_SYSTEM, &snippet, 512).await {
        Some(t) => t,
        None => return vec![],
    };
    let raw: Vec<RawFact> =
        serde_json::from_str(crate::core::llm::json_slice(&text)).unwrap_or_default();

    raw.into_iter()
        .filter(|f| !f.content.is_empty())
        .map(|f| ExtractedFact {
            content: f.content,
            category: f.category,
            confidence: f.confidence.clamp(0.0, 1.0),
        })
        .collect()
}

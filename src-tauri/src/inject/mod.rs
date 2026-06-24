pub mod health;

use crate::core::types::{CompletionRequest, Message};

const MAX_HISTORY_MESSAGES: usize = 20;
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct Injector {
    pub system_prompt: Option<String>,
    pub max_tokens: u32,
}

impl Injector {
    pub fn new(system_prompt: Option<String>) -> Self {
        Self {
            system_prompt,
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    /// Assemble a CompletionRequest. Prepends known_facts to the system prompt.
    pub fn assemble(
        &self,
        history: Vec<Message>,
        model: String,
        known_facts: &[String],
    ) -> CompletionRequest {
        let messages = if history.len() > MAX_HISTORY_MESSAGES {
            history[history.len() - MAX_HISTORY_MESSAGES..].to_vec()
        } else {
            history
        };
        let system_prompt = build_system_prompt(self.system_prompt.as_deref(), known_facts);
        CompletionRequest {
            model,
            messages,
            system_prompt,
            max_tokens: self.max_tokens,
        }
    }
}

fn build_system_prompt(base: Option<&str>, facts: &[String]) -> Option<String> {
    if facts.is_empty() {
        return base.map(str::to_owned);
    }
    let facts_block = format!(
        "Known facts about the user:\n{}",
        facts
            .iter()
            .map(|f| format!("- {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Some(match base {
        Some(sp) if !sp.is_empty() => format!("{sp}\n\n{facts_block}"),
        _ => facts_block,
    })
}

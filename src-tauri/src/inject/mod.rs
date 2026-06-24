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

    pub fn assemble(
        &self,
        history: Vec<Message>,
        model: String,
        known_facts: &[String],
        open_tasks: &[String],
    ) -> CompletionRequest {
        let messages = if history.len() > MAX_HISTORY_MESSAGES {
            history[history.len() - MAX_HISTORY_MESSAGES..].to_vec()
        } else {
            history
        };
        let system_prompt =
            build_system_prompt(self.system_prompt.as_deref(), known_facts, open_tasks);
        CompletionRequest {
            model,
            messages,
            system_prompt,
            max_tokens: self.max_tokens,
        }
    }
}

fn build_system_prompt(base: Option<&str>, facts: &[String], tasks: &[String]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(sp) = base.filter(|s| !s.is_empty()) {
        parts.push(sp.to_owned());
    }
    if !facts.is_empty() {
        parts.push(format!(
            "Known facts about the user:\n{}",
            facts
                .iter()
                .map(|f| format!("- {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !tasks.is_empty() {
        parts.push(format!(
            "Open tasks:\n{}",
            tasks
                .iter()
                .map(|t| format!("- {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

pub mod health;

use crate::core::types::{CompletionRequest, Message};

/// Token budget fractions (Phase 1: history only, no memory yet).
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

    /// Assemble a CompletionRequest from conversation history + new user message.
    /// Prunes history to MAX_HISTORY_MESSAGES (HISTSUM Phase 1: simple truncation).
    pub fn assemble(&self, history: Vec<Message>, model: String) -> CompletionRequest {
        let messages = if history.len() > MAX_HISTORY_MESSAGES {
            history[history.len() - MAX_HISTORY_MESSAGES..].to_vec()
        } else {
            history
        };

        CompletionRequest {
            model,
            messages,
            system_prompt: self.system_prompt.clone(),
            max_tokens: self.max_tokens,
        }
    }
}

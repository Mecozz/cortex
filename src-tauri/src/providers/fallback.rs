use serde::{Deserialize, Serialize};

/// What to do when the active provider fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Surface the error to the user immediately.
    HardFail,
    /// Return an empty response without notifying the user.
    Silent,
    /// Show an error banner but keep the conversation alive.
    Transparent,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::HardFail
    }
}

impl FallbackPolicy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "silent" => Self::Silent,
            "transparent" => Self::Transparent,
            _ => Self::HardFail,
        }
    }
}

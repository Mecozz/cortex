use serde::{Deserialize, Serialize};

/// What to do when the active provider fails.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Surface the error to the user immediately.
    #[default]
    HardFail,
    /// Return an empty response without notifying the user.
    Silent,
    /// Show an error banner but keep the conversation alive.
    Transparent,
}

impl std::str::FromStr for FallbackPolicy {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "silent" => Self::Silent,
            "transparent" => Self::Transparent,
            _ => Self::HardFail,
        })
    }
}

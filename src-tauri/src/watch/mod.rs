pub mod health;

use serde::Serialize;
use std::collections::HashMap;

const FAILURE_THRESHOLD: u32 = 3;
/// How long a tripped breaker stays open before allowing a trial call (half-open).
const COOLDOWN_SECS: i64 = 60;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Debug, Default)]
pub struct CircuitBreaker {
    failures: HashMap<String, u32>,
    /// module -> unix time until which it stays disabled.
    disabled_until: HashMap<String, i64>,
}

impl CircuitBreaker {
    pub fn record_failure(&mut self, module: &str) {
        let n = self.failures.entry(module.to_owned()).or_default();
        *n += 1;
        if *n >= FAILURE_THRESHOLD {
            // Trip the breaker for a cooldown window. After it elapses, one trial
            // call is allowed (half-open): success clears it, failure re-trips.
            self.disabled_until
                .insert(module.to_owned(), now_secs() + COOLDOWN_SECS);
        }
    }

    pub fn record_success(&mut self, module: &str) {
        self.failures.remove(module);
        self.disabled_until.remove(module);
    }

    pub fn is_disabled(&self, module: &str) -> bool {
        self.disabled_until
            .get(module)
            .map(|&until| now_secs() < until)
            .unwrap_or(false)
    }

    pub fn failure_count(&self, module: &str) -> u32 {
        self.failures.get(module).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrainStatus {
    pub overall: String,
    pub modules: Vec<ModuleHealth>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleHealth {
    pub module: String,
    pub status: String,
    pub message: Option<String>,
    pub failures: u32,
    pub disabled: bool,
}

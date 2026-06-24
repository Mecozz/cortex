pub mod health;

use serde::Serialize;
use std::collections::{HashMap, HashSet};

const FAILURE_THRESHOLD: u32 = 3;

#[derive(Debug, Default)]
pub struct CircuitBreaker {
    failures: HashMap<String, u32>,
    disabled: HashSet<String>,
}

impl CircuitBreaker {
    pub fn record_failure(&mut self, module: &str) {
        let n = self.failures.entry(module.to_owned()).or_default();
        *n += 1;
        if *n >= FAILURE_THRESHOLD {
            self.disabled.insert(module.to_owned());
        }
    }

    pub fn record_success(&mut self, module: &str) {
        self.failures.remove(module);
        self.disabled.remove(module);
    }

    pub fn is_disabled(&self, module: &str) -> bool {
        self.disabled.contains(module)
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

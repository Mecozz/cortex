//! Health contract -- every module must implement this trait.
//! WATCH aggregates health status from all modules.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub module: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub checked_at: u64,
}

impl HealthReport {
    pub fn new(module: &str, status: HealthStatus, message: Option<String>) -> Self {
        Self {
            module: module.to_string(),
            status,
            message,
            checked_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

pub trait HealthCheck: Send + Sync {
    fn module_name(&self) -> &str;
    fn health(&self) -> HealthReport;
}

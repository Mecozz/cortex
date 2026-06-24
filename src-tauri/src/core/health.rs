//! Health contract -- every module must implement this trait.
//! WATCH aggregates health status from all modules.

use serde::{Deserialize, Serialize};

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

pub trait HealthCheck: Send + Sync {
    fn module_name(&self) -> &str;
    fn health(&self) -> HealthReport;
}
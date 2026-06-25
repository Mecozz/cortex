use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

/// Memory health, parameterised with live counts (set by get_brain_status).
pub struct MemoryHealth {
    pub facts: i64,
    pub episodic: i64,
}

impl HealthCheck for MemoryHealth {
    fn module_name(&self) -> &str {
        "memory"
    }

    fn health(&self) -> HealthReport {
        let status = if self.facts > 0 || self.episodic > 0 {
            HealthStatus::Green
        } else {
            HealthStatus::Yellow
        };
        let message = format!("{} facts, {} episodic messages", self.facts, self.episodic);
        HealthReport::new(self.module_name(), status, Some(message))
    }
}

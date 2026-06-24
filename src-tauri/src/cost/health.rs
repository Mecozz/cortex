use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct CostHealth;

impl HealthCheck for CostHealth {
    fn module_name(&self) -> &str {
        "cost"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(self.module_name(), HealthStatus::Green, None)
    }
}

pub fn reset() {}

use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct InjectHealth;

impl HealthCheck for InjectHealth {
    fn module_name(&self) -> &str {
        "inject"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(self.module_name(), HealthStatus::Green, None)
    }
}

pub fn reset() {}

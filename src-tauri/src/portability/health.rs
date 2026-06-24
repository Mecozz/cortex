use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct PortabilityHealth;

impl HealthCheck for PortabilityHealth {
    fn module_name(&self) -> &str {
        "portability"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("EXPORT/IMPORT ready".into()),
        )
    }
}

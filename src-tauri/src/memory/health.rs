use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct MemoryHealth;

impl HealthCheck for MemoryHealth {
    fn module_name(&self) -> &str {
        "memory"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("INSTCAP + CONF + PASS1 active".into()),
        )
    }
}

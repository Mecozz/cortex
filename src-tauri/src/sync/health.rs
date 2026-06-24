use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct SyncHealth;

impl HealthCheck for SyncHealth {
    fn module_name(&self) -> &str {
        "sync"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("SFOLDER sync active".into()),
        )
    }
}

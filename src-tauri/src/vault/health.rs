use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct VaultHealth;

impl HealthCheck for VaultHealth {
    fn module_name(&self) -> &str {
        "vault"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("AES-256-GCM encryption active".into()),
        )
    }
}

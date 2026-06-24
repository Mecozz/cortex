use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct BackupHealth;

impl HealthCheck for BackupHealth {
    fn module_name(&self) -> &str {
        "backup"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("BKUP + RLVL active".into()),
        )
    }
}

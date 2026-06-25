use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

/// Backup health, parameterised with the live backup count.
pub struct BackupHealth {
    pub count: usize,
}

impl HealthCheck for BackupHealth {
    fn module_name(&self) -> &str {
        "backup"
    }

    fn health(&self) -> HealthReport {
        let (status, message) = if self.count > 0 {
            (HealthStatus::Green, format!("{} backups", self.count))
        } else {
            (HealthStatus::Yellow, "no backups yet".to_string())
        };
        HealthReport::new(self.module_name(), status, Some(message))
    }
}

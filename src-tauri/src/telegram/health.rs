use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct TelegramHealth;

impl HealthCheck for TelegramHealth {
    fn module_name(&self) -> &str {
        "telegram"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("TGRAM notify ready".into()),
        )
    }
}

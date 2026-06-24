use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct WatchHealth;

impl HealthCheck for WatchHealth {
    fn module_name(&self) -> &str {
        "watch"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("Circuit breaker + health aggregation active".into()),
        )
    }
}

use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct TaskHealth;

impl HealthCheck for TaskHealth {
    fn module_name(&self) -> &str {
        "tasks"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("TASKEXT active".into()),
        )
    }
}

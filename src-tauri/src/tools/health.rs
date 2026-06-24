use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct ToolsHealth;

impl HealthCheck for ToolsHealth {
    fn module_name(&self) -> &str {
        "tools"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("TCAT/TOOLRUN/FORGE ready".into()),
        )
    }
}

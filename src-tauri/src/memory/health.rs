use crate::core::health::{HealthReport, HealthStatus, Module};

pub struct MemoryHealth;

impl Module for MemoryHealth {
    fn module_name(&self) -> &str {
        "memory"
    }

    fn health(&self) -> HealthReport {
        HealthReport {
            module: self.module_name().into(),
            status: HealthStatus::Ok,
            message: "INSTCAP + CONF + PASS1 active".into(),
        }
    }
}

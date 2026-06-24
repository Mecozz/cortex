use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct ProvidersHealth {
    pub cloud_available: bool,
    pub local_available: bool,
}

impl HealthCheck for ProvidersHealth {
    fn module_name(&self) -> &str {
        "providers"
    }

    fn health(&self) -> HealthReport {
        let status = if self.cloud_available || self.local_available {
            HealthStatus::Green
        } else {
            HealthStatus::Yellow
        };

        let message = match (self.cloud_available, self.local_available) {
            (true, true) => "cloud + local available".into(),
            (true, false) => "cloud only".into(),
            (false, true) => "local only (Ollama)".into(),
            (false, false) => "no provider configured".into(),
        };

        HealthReport::new(self.module_name(), status, Some(message))
    }
}

pub fn reset() {}

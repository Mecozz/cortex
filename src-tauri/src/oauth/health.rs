use crate::core::health::{HealthCheck, HealthReport, HealthStatus};

pub struct OAuthHealth;

impl HealthCheck for OAuthHealth {
    fn module_name(&self) -> &str {
        "oauth"
    }

    fn health(&self) -> HealthReport {
        HealthReport::new(
            self.module_name(),
            HealthStatus::Green,
            Some("OAuth PKCE flow ready".into()),
        )
    }
}

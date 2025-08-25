//! Health check endpoint implementation

use crate::config::HttpConfig;
use elif_core::container::IocContainer;
use serde_json::json;
use std::sync::Arc;

/// Default health check handler
pub async fn health_check_handler(
    _container: Arc<IocContainer>,
    _config: HttpConfig,
) -> axum::response::Json<serde_json::Value> {
    let response = json!({
        "status": "healthy",
        "framework": "Elif.rs",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "server": {
            "ready": true,
            "uptime": "N/A"
        }
    });

    axum::response::Json(response)
}

/// Health check response structure
#[derive(serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub framework: String,
    pub version: String,
    pub timestamp: u64,
    pub server: ServerStatus,
}

#[derive(serde::Serialize)]
pub struct ServerStatus {
    pub ready: bool,
    pub uptime: String,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            status: "healthy".to_string(),
            framework: "Elif.rs".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            server: ServerStatus {
                ready: true,
                uptime: "N/A".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::create_test_container;

    #[tokio::test]
    async fn test_health_check_handler() {
        let container = create_test_container();
        let config = HttpConfig::default();

        let response = health_check_handler(container, config).await;
        // Test that response is properly formatted JSON
        assert!(response.0.get("status").is_some());
        assert_eq!(response.0["status"], "healthy");
    }

    #[test]
    fn test_health_status_default() {
        let status = HealthStatus::default();
        assert_eq!(status.status, "healthy");
        assert_eq!(status.framework, "Elif.rs");
        assert!(status.server.ready);
    }
}

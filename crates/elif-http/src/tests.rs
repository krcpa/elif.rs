//! Tests for HTTP server functionality

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HttpConfig, Server};
    use elif_core::{
        Container,
        container::test_implementations::*,
        app_config::{AppConfigTrait},
    };
    use std::sync::Arc;

    fn create_test_container() -> Arc<Container> {
        let config = Arc::new(create_test_config());
        let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
        
        Container::builder()
            .config(config)
            .database(database)
            .build()
            .unwrap()
            .into()
    }

    #[test]
    fn test_http_config_defaults() {
        let config = HttpConfig::default();
        
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.keep_alive_timeout_secs, 75);
        assert_eq!(config.max_request_size, 16 * 1024 * 1024);
        assert!(config.enable_tracing);
        assert_eq!(config.health_check_path, "/health");
        assert_eq!(config.shutdown_timeout_secs, 10);
    }

    #[test]
    fn test_http_config_from_env() {
        std::env::remove_var("HTTP_REQUEST_TIMEOUT");
        std::env::remove_var("HTTP_KEEP_ALIVE_TIMEOUT");
        
        let config = HttpConfig::from_env().unwrap();
        config.validate().unwrap();
        
        // Test duration helpers
        assert_eq!(config.request_timeout().as_secs(), 30);
        assert_eq!(config.keep_alive_timeout().as_secs(), 75);
        assert_eq!(config.shutdown_timeout().as_secs(), 10);
    }

    #[test]
    fn test_http_config_validation() {
        let mut config = HttpConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid request timeout
        config.request_timeout_secs = 0;
        assert!(config.validate().is_err());

        // Test invalid health check path
        config = HttpConfig::default();
        config.health_check_path = "no-slash".to_string();
        assert!(config.validate().is_err());

        config.health_check_path = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_creation_with_container() {
        let container = create_test_container();
        let http_config = HttpConfig::default();

        let server = Server::with_container(container, http_config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_with_invalid_address() {
        let container = create_test_container();
        let http_config = HttpConfig::default();
        
        let server = Server::with_container(container, http_config).unwrap();
        
        // Test with invalid address - this should fail during listen, not creation
        // For now, just verify server can be created with valid configuration
        // Invalid address testing would be done in integration tests with actual listen() calls
    }

    #[tokio::test]
    async fn test_health_check_endpoint() {
        use crate::server::health_check;
        
        let container = create_test_container();
        let config = HttpConfig::default();
        
        let response = health_check(container, config).await;
        let value = response.0;
        
        assert_eq!(value["status"], "healthy");
        assert_eq!(value["framework"], "Elif.rs");
        assert!(value["timestamp"].is_number());
    }

    #[test]
    fn test_http_error_types() {
        use crate::error::HttpError;
        use axum::http::StatusCode;
        
        let startup_error = HttpError::startup("Failed to bind");
        assert_eq!(startup_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(startup_error.error_code(), "SERVER_STARTUP_FAILED");
        
        let timeout_error = HttpError::RequestTimeout;
        assert_eq!(timeout_error.status_code(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(timeout_error.error_code(), "REQUEST_TIMEOUT");
        
        let bad_request = HttpError::bad_request("Invalid input");
        assert_eq!(bad_request.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(bad_request.error_code(), "BAD_REQUEST");
    }

    #[test]
    fn test_error_conversions() {
        use crate::error::HttpError;
        use elif_core::app_config::ConfigError;
        
        // Test ConfigError conversion
        let config_error = ConfigError::MissingEnvVar {
            var: "TEST_VAR".to_string(),
        };
        let http_error: HttpError = config_error.into();
        assert!(matches!(http_error, HttpError::ConfigError { .. }));
        
        // Test IO error conversion
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let http_error: HttpError = io_error.into();
        assert!(matches!(http_error, HttpError::InternalError { .. }));
    }
}
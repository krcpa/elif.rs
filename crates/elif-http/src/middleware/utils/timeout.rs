//! # Timeout Middleware
//!
//! Framework middleware for request timeout handling.
//! Replaces tower-http TimeoutLayer with framework-native implementation.

use crate::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
};
use serde_json;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, warn};

/// Configuration for timeout middleware
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Request timeout duration
    pub timeout: Duration,
    /// Whether to log timeout events
    pub log_timeouts: bool,
    /// Custom timeout error message
    pub timeout_message: String,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            log_timeouts: true,
            timeout_message: "Request timed out".to_string(),
        }
    }
}

impl TimeoutConfig {
    /// Create new timeout configuration
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            ..Default::default()
        }
    }

    /// Set timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable timeout logging
    pub fn with_logging(mut self, log_timeouts: bool) -> Self {
        self.log_timeouts = log_timeouts;
        self
    }

    /// Set custom timeout error message
    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.timeout_message = message.into();
        self
    }
}

/// Framework timeout middleware for HTTP requests
#[derive(Debug)]
pub struct TimeoutMiddleware {
    config: TimeoutConfig,
}

impl TimeoutMiddleware {
    /// Create new timeout middleware with default 30 second timeout
    pub fn new() -> Self {
        Self {
            config: TimeoutConfig::default(),
        }
    }

    /// Create timeout middleware with specific duration
    pub fn with_duration(timeout: Duration) -> Self {
        Self {
            config: TimeoutConfig::new(timeout),
        }
    }

    /// Create timeout middleware with custom configuration
    pub fn with_config(config: TimeoutConfig) -> Self {
        Self { config }
    }

    /// Set timeout duration (builder pattern)
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.config = self.config.with_timeout(duration);
        self
    }

    /// Enable or disable logging (builder pattern)
    pub fn logging(mut self, enabled: bool) -> Self {
        self.config = self.config.with_logging(enabled);
        self
    }

    /// Set custom timeout message (builder pattern)
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.config = self.config.with_message(message);
        self
    }

    /// Get timeout duration
    pub fn duration(&self) -> Duration {
        self.config.timeout
    }
}

impl Default for TimeoutMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TimeoutMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        let timeout_duration = self.config.timeout;
        let log_timeouts = self.config.log_timeouts;
        let timeout_message = self.config.timeout_message.clone();

        Box::pin(async move {
            // Apply timeout to the entire middleware chain
            match timeout(timeout_duration, next.run(request)).await {
                Ok(response) => {
                    // Check if response indicates timeout and log if enabled
                    if response.status_code() == ElifStatusCode::REQUEST_TIMEOUT && log_timeouts {
                        warn!("Request timed out after {:?}", timeout_duration);
                    }
                    response
                }
                Err(_) => {
                    // Timeout occurred
                    if log_timeouts {
                        error!(
                            "Request timed out after {:?}: {}",
                            timeout_duration, timeout_message
                        );
                    }

                    ElifResponse::with_status(ElifStatusCode::REQUEST_TIMEOUT).json_value(
                        serde_json::json!({
                            "error": {
                                "code": "REQUEST_TIMEOUT",
                                "message": &timeout_message,
                                "timeout_duration_secs": timeout_duration.as_secs()
                            }
                        }),
                    )
                }
            }
        })
    }

    fn name(&self) -> &'static str {
        "TimeoutMiddleware"
    }
}

/// Timeout information stored in request extensions
#[derive(Debug, Clone)]
pub struct TimeoutInfo {
    pub duration: Duration,
    pub message: String,
}

/// Helper function to apply timeout to a future
pub async fn apply_timeout<F, T>(
    future: F,
    duration: Duration,
    timeout_message: &str,
) -> Result<T, ElifResponse>
where
    F: std::future::Future<Output = T>,
{
    match timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => {
            error!(
                "Request timed out after {:?}: {}",
                duration, timeout_message
            );
            Err(
                ElifResponse::with_status(ElifStatusCode::REQUEST_TIMEOUT).json_value(
                    serde_json::json!({
                        "error": {
                            "code": "REQUEST_TIMEOUT",
                            "message": timeout_message,
                            "timeout_duration_secs": duration.as_secs()
                        }
                    }),
                ),
            )
        }
    }
}

// TimeoutHandler removed - use TimeoutMiddleware with V2 system instead

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{middleware::v2::Next, request::ElifRequest};
    use std::time::Duration;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_timeout_middleware_fast_response() {
        let middleware = TimeoutMiddleware::with_duration(Duration::from_secs(1));

        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );

        let next = Next::new(|_req| Box::pin(async { ElifResponse::ok().text("Fast response") }));

        let response = middleware.handle(request, next).await;
        assert_eq!(response.status_code(), crate::response::ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_timeout_middleware_slow_response() {
        let middleware = TimeoutMiddleware::with_duration(Duration::from_millis(100));

        let request = ElifRequest::new(
            crate::request::ElifMethod::GET,
            "/test".parse().unwrap(),
            crate::response::headers::ElifHeaderMap::new(),
        );

        let next = Next::new(|_req| {
            Box::pin(async {
                // Slow response that will timeout
                sleep(TokioDuration::from_millis(200)).await;
                ElifResponse::ok().text("Should not reach here")
            })
        });

        let response = middleware.handle(request, next).await;
        assert_eq!(
            response.status_code(),
            crate::response::ElifStatusCode::REQUEST_TIMEOUT
        );
    }

    #[tokio::test]
    async fn test_timeout_middleware_custom_config() {
        let config = TimeoutConfig::new(Duration::from_secs(60))
            .with_logging(false)
            .with_message("Custom timeout");

        let middleware = TimeoutMiddleware::with_config(config);

        assert_eq!(middleware.duration(), Duration::from_secs(60));
        assert!(!middleware.config.log_timeouts);
        assert_eq!(middleware.config.timeout_message, "Custom timeout");
    }

    #[tokio::test]
    async fn test_timeout_middleware_builder() {
        let middleware = TimeoutMiddleware::new()
            .timeout(Duration::from_secs(45))
            .logging(true)
            .message("Builder timeout");

        assert_eq!(middleware.duration(), Duration::from_secs(45));
        assert!(middleware.config.log_timeouts);
        assert_eq!(middleware.config.timeout_message, "Builder timeout");
    }

    #[tokio::test]
    async fn test_timeout_middleware_name() {
        let middleware = TimeoutMiddleware::new();
        assert_eq!(middleware.name(), "TimeoutMiddleware");
    }

    #[tokio::test]
    async fn test_apply_timeout_success() {
        let future = async { "success" };
        let result = apply_timeout(future, Duration::from_secs(1), "test timeout").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_apply_timeout_failure() {
        let future = async {
            sleep(TokioDuration::from_secs(2)).await;
            "should not reach here"
        };

        let result = apply_timeout(future, Duration::from_millis(100), "test timeout").await;
        assert!(result.is_err());

        // Verify it's a timeout response
        let response = result.unwrap_err();
        assert_eq!(
            response.status_code(),
            crate::response::ElifStatusCode::REQUEST_TIMEOUT
        );
    }

    #[tokio::test]
    async fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.log_timeouts);
        assert_eq!(config.timeout_message, "Request timed out");
    }
}

//! # Timeout Middleware
//!
//! Framework middleware for request timeout handling.
//! Replaces tower-http TimeoutLayer with framework-native implementation.

use std::time::Duration;
use tokio::time::timeout;
use axum::{
    extract::Request,
    response::{Response, IntoResponse},
    http::StatusCode,
};
use tracing::{warn, error};

use crate::{
    middleware::{Middleware, BoxFuture},
    HttpError,
};

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

    /// Create timeout error response
    fn timeout_response(&self) -> Response {
        let error = HttpError::timeout(&self.config.timeout_message);
        error.into_response()
    }
}

impl Default for TimeoutMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TimeoutMiddleware {
    fn process_request<'a>(
        &'a self,
        request: Request
    ) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // Store timeout duration in request extensions for downstream middleware
            // This allows handlers to know the timeout that's been applied
            let mut request = request;
            request.extensions_mut().insert(TimeoutInfo {
                duration: self.config.timeout,
                message: self.config.timeout_message.clone(),
            });

            Ok(request)
        })
    }

    fn process_response<'a>(
        &'a self,
        response: Response
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // For timeout middleware, response processing is mainly for logging
            // The actual timeout handling happens at the handler level or higher
            
            if response.status() == StatusCode::REQUEST_TIMEOUT && self.config.log_timeouts {
                warn!("Request timed out after {:?}", self.config.timeout);
            }

            response
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
) -> Result<T, Response>
where
    F: std::future::Future<Output = T>,
{
    match timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => {
            error!("Request timed out after {:?}: {}", duration, timeout_message);
            let error = HttpError::timeout(timeout_message);
            Err(error.into_response())
        }
    }
}

/// Timeout middleware wrapper that can be applied to handlers
pub struct TimeoutHandler<F> {
    handler: F,
    duration: Duration,
    message: String,
}

impl<F> TimeoutHandler<F> {
    pub fn new(handler: F, duration: Duration) -> Self {
        Self {
            handler,
            duration,
            message: "Request timed out".to_string(),
        }
    }

    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = message.into();
        self
    }
}

impl<F, Fut, T> tower::Service<Request> for TimeoutHandler<F>
where
    F: tower::Service<Request, Response = T, Future = Fut> + Clone + Send + 'static,
    Fut: std::future::Future<Output = Result<T, F::Error>> + Send + 'static,
    T: axum::response::IntoResponse,
{
    type Response = Response;
    type Error = Response;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        match self.handler.poll_ready(cx) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(Err(_)) => {
                let error = HttpError::internal("Handler not ready");
                std::task::Poll::Ready(Err(error.into_response()))
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let handler = self.handler.clone();
        let mut handler = handler;
        let duration = self.duration;
        let message = self.message.clone();

        Box::pin(async move {
            match timeout(duration, handler.call(request)).await {
                Ok(Ok(response)) => Ok(response.into_response()),
                Ok(Err(_)) => {
                    let error = HttpError::internal("Handler error");
                    Err(error.into_response())
                },
                Err(_) => {
                    error!("Request timed out after {:?}: {}", duration, message);
                    let error = HttpError::timeout(&message);
                    Err(error.into_response())
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode};
    use tokio::time::{sleep, Duration as TokioDuration};
    use std::time::Duration;

    #[tokio::test]
    async fn test_timeout_middleware_basic() {
        let middleware = TimeoutMiddleware::new();
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        assert!(result.is_ok());

        let processed_request = result.unwrap();
        
        // Check that timeout info was added to extensions
        let timeout_info = processed_request.extensions().get::<TimeoutInfo>();
        assert!(timeout_info.is_some());
        
        let timeout_info = timeout_info.unwrap();
        assert_eq!(timeout_info.duration, Duration::from_secs(30));
        assert_eq!(timeout_info.message, "Request timed out");
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
    async fn test_timeout_middleware_response() {
        let middleware = TimeoutMiddleware::new();
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::empty())
            .unwrap();

        let processed_response = middleware.process_response(response).await;
        assert_eq!(processed_response.status(), StatusCode::OK);
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
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.log_timeouts);
        assert_eq!(config.timeout_message, "Request timed out");
    }

    #[tokio::test]
    async fn test_timeout_info_extension() {
        let middleware = TimeoutMiddleware::with_duration(Duration::from_secs(15));
        
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = middleware.process_request(request).await;
        let processed_request = result.unwrap();
        
        let timeout_info = processed_request.extensions().get::<TimeoutInfo>().unwrap();
        assert_eq!(timeout_info.duration, Duration::from_secs(15));
        assert_eq!(timeout_info.message, "Request timed out");
    }
}
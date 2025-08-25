//! # Structured Logging Integration
//!
//! Complete structured logging system for the elif.rs framework with
//! JSON output, tracing integration, and production-ready configuration.

use serde_json::{json, Value};
use std::io;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Logging configuration for the elif.rs framework
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level filter (e.g., "info", "debug", "warn")
    pub level: String,
    /// Enable JSON structured logging (vs plain text)
    pub json_format: bool,
    /// Enable pretty printing for development
    pub pretty_print: bool,
    /// Include file and line number information
    pub include_location: bool,
    /// Include timestamp in logs
    pub include_timestamp: bool,
    /// Custom fields to include in all log entries
    pub global_fields: serde_json::Map<String, Value>,
    /// Environment filter (supports complex filters like "elif=debug,tower=info")
    pub env_filter: Option<String>,
    /// Service name to include in all logs
    pub service_name: Option<String>,
    /// Service version to include in all logs
    pub service_version: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            pretty_print: true,
            include_location: false,
            include_timestamp: true,
            global_fields: serde_json::Map::new(),
            env_filter: None,
            service_name: None,
            service_version: None,
        }
    }
}

impl LoggingConfig {
    /// Create production logging configuration
    pub fn production() -> Self {
        Self {
            level: "info".to_string(),
            json_format: true,
            pretty_print: false,
            include_location: false,
            include_timestamp: true,
            global_fields: {
                let mut fields = serde_json::Map::new();
                fields.insert("env".to_string(), json!("production"));
                fields
            },
            env_filter: Some("elif=info,tower=warn,axum=warn".to_string()),
            service_name: None,
            service_version: None,
        }
    }

    /// Create development logging configuration
    pub fn development() -> Self {
        Self {
            level: "debug".to_string(),
            json_format: false,
            pretty_print: true,
            include_location: true,
            include_timestamp: true,
            global_fields: {
                let mut fields = serde_json::Map::new();
                fields.insert("env".to_string(), json!("development"));
                fields
            },
            env_filter: Some("elif=debug,tower=debug,axum=debug".to_string()),
            service_name: None,
            service_version: None,
        }
    }

    /// Create test logging configuration (minimal output)
    pub fn test() -> Self {
        Self {
            level: "error".to_string(),
            json_format: false,
            pretty_print: false,
            include_location: false,
            include_timestamp: false,
            global_fields: {
                let mut fields = serde_json::Map::new();
                fields.insert("env".to_string(), json!("test"));
                fields
            },
            env_filter: Some("elif=error".to_string()),
            service_name: None,
            service_version: None,
        }
    }

    /// Add a global field to include in all log entries
    pub fn with_global_field<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.global_fields.insert(key.into(), value.into());
        self
    }

    /// Set service name and version
    pub fn with_service(mut self, name: &str, version: &str) -> Self {
        self.service_name = Some(name.to_string());
        self.service_version = Some(version.to_string());
        self
    }

    /// Set environment filter
    pub fn with_env_filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.env_filter = Some(filter.into());
        self
    }
}

/// Initialize structured logging for the application
pub fn init_logging(config: LoggingConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = config.env_filter.as_deref().unwrap_or(&config.level);

    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(env_filter))?;

    if config.json_format {
        // JSON structured logging
        tracing_subscriber::registry()
            .with(filter)
            .with(Layer::new().with_writer(io::stdout).json())
            .init();
    } else if config.pretty_print {
        // Pretty text logging
        tracing_subscriber::registry()
            .with(filter)
            .with(Layer::new().with_writer(io::stdout).pretty())
            .init();
    } else {
        // Plain text logging
        tracing_subscriber::registry()
            .with(filter)
            .with(Layer::new().with_writer(io::stdout))
            .init();
    }

    // Log initialization message with global fields
    if !config.global_fields.is_empty() {
        let mut init_msg = json!({
            "message": "Structured logging initialized",
            "config": {
                "level": config.level,
                "json_format": config.json_format,
                "pretty_print": config.pretty_print,
                "include_location": config.include_location,
                "include_timestamp": config.include_timestamp,
            }
        });

        // Add service info if available
        if let Some(name) = config.service_name {
            init_msg["service_name"] = json!(name);
        }
        if let Some(version) = config.service_version {
            init_msg["service_version"] = json!(version);
        }

        // Add global fields
        for (key, value) in config.global_fields {
            init_msg[key] = value;
        }

        tracing::info!(target: "elif::logging", "{}", init_msg);
    } else {
        tracing::info!(
            target: "elif::logging",
            "Structured logging initialized (level: {}, format: {})",
            config.level,
            if config.json_format { "JSON" } else { "text" }
        );
    }

    Ok(())
}

/// Convenience macro for structured logging with context
#[macro_export]
macro_rules! log_with_context {
    ($level:expr, $($field:tt)*) => {
        tracing::event!($level, $($field)*)
    };
}

/// Convenience macro for structured info logging
#[macro_export]
macro_rules! info_structured {
    ($($field:tt)*) => {
        $crate::log_with_context!(tracing::Level::INFO, $($field)*)
    };
}

/// Convenience macro for structured error logging
#[macro_export]
macro_rules! error_structured {
    ($($field:tt)*) => {
        $crate::log_with_context!(tracing::Level::ERROR, $($field)*)
    };
}

/// Convenience macro for structured debug logging
#[macro_export]
macro_rules! debug_structured {
    ($($field:tt)*) => {
        $crate::log_with_context!(tracing::Level::DEBUG, $($field)*)
    };
}

/// Log application startup with system information
pub fn log_startup_info(service_name: &str, service_version: &str) {
    let startup_info = json!({
        "event": "application_startup",
        "service": service_name,
        "version": service_version,
        "pid": std::process::id(),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    tracing::info!(target: "elif::startup", "{}", startup_info);
}

/// Log application shutdown
pub fn log_shutdown_info(service_name: &str) {
    let shutdown_info = json!({
        "event": "application_shutdown",
        "service": service_name,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    tracing::info!(target: "elif::shutdown", "{}", shutdown_info);
}

/// Create a logging context for request tracking
#[derive(Debug, Clone)]
pub struct LoggingContext {
    pub correlation_id: String,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub custom_fields: serde_json::Map<String, Value>,
}

impl LoggingContext {
    pub fn new(correlation_id: String) -> Self {
        Self {
            correlation_id,
            request_id: None,
            user_id: None,
            session_id: None,
            custom_fields: serde_json::Map::new(),
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_custom_field<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.custom_fields.insert(key.into(), value.into());
        self
    }

    /// Create a JSON object with all context fields
    pub fn to_json(&self) -> Value {
        let mut context = json!({
            "correlation_id": self.correlation_id,
        });

        if let Some(request_id) = &self.request_id {
            context["request_id"] = json!(request_id);
        }

        if let Some(user_id) = &self.user_id {
            context["user_id"] = json!(user_id);
        }

        if let Some(session_id) = &self.session_id {
            context["session_id"] = json!(session_id);
        }

        for (key, value) in &self.custom_fields {
            context[key] = value.clone();
        }

        context
    }
}

/// Structured logging utilities for common scenarios
pub mod structured {
    use super::*;
    use tracing::{debug, error, info, warn};

    /// Log an HTTP request
    pub fn log_http_request(
        context: &LoggingContext,
        method: &str,
        path: &str,
        status: u16,
        duration_ms: u128,
        user_agent: Option<&str>,
    ) {
        let mut log_data = json!({
            "event": "http_request",
            "method": method,
            "path": path,
            "status": status,
            "duration_ms": duration_ms,
        });

        // Add context
        let context_json = context.to_json();
        for (key, value) in context_json.as_object().unwrap() {
            log_data[key] = value.clone();
        }

        if let Some(ua) = user_agent {
            log_data["user_agent"] = json!(ua);
        }

        if status >= 500 {
            error!(target: "elif::http", "{}", log_data);
        } else if status >= 400 {
            warn!(target: "elif::http", "{}", log_data);
        } else {
            info!(target: "elif::http", "{}", log_data);
        }
    }

    /// Log a database query
    pub fn log_database_query(
        context: &LoggingContext,
        query: &str,
        duration_ms: u128,
        affected_rows: Option<u64>,
    ) {
        let mut log_data = json!({
            "event": "database_query",
            "query": query,
            "duration_ms": duration_ms,
        });

        // Add context
        let context_json = context.to_json();
        for (key, value) in context_json.as_object().unwrap() {
            log_data[key] = value.clone();
        }

        if let Some(rows) = affected_rows {
            log_data["affected_rows"] = json!(rows);
        }

        if duration_ms > 1000 {
            warn!(target: "elif::database", "Slow query: {}", log_data);
        } else {
            debug!(target: "elif::database", "{}", log_data);
        }
    }

    /// Log an application error
    pub fn log_application_error(
        context: &LoggingContext,
        error_type: &str,
        error_message: &str,
        error_details: Option<&str>,
    ) {
        let mut log_data = json!({
            "event": "application_error",
            "error_type": error_type,
            "error_message": error_message,
        });

        // Add context
        let context_json = context.to_json();
        for (key, value) in context_json.as_object().unwrap() {
            log_data[key] = value.clone();
        }

        if let Some(details) = error_details {
            log_data["error_details"] = json!(details);
        }

        error!(target: "elif::error", "{}", log_data);
    }

    /// Log a security event
    pub fn log_security_event(
        context: &LoggingContext,
        event_type: &str,
        severity: &str,
        details: &str,
        ip_address: Option<&str>,
    ) {
        let mut log_data = json!({
            "event": "security_event",
            "event_type": event_type,
            "severity": severity,
            "details": details,
        });

        // Add context
        let context_json = context.to_json();
        for (key, value) in context_json.as_object().unwrap() {
            log_data[key] = value.clone();
        }

        if let Some(ip) = ip_address {
            log_data["ip_address"] = json!(ip);
        }

        match severity {
            "high" | "critical" => error!(target: "elif::security", "{}", log_data),
            "medium" => warn!(target: "elif::security", "{}", log_data),
            _ => info!(target: "elif::security", "{}", log_data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_presets() {
        let prod = LoggingConfig::production();
        assert!(prod.json_format);
        assert!(!prod.pretty_print);
        assert_eq!(prod.level, "info");
        assert!(prod.global_fields.contains_key("env"));

        let dev = LoggingConfig::development();
        assert!(!dev.json_format);
        assert!(dev.pretty_print);
        assert_eq!(dev.level, "debug");
        assert!(dev.include_location);

        let test = LoggingConfig::test();
        assert_eq!(test.level, "error");
        assert!(!test.include_timestamp);
    }

    #[test]
    fn test_logging_config_builder() {
        let config = LoggingConfig::default()
            .with_global_field("app", "test-app")
            .with_service("test-service", "1.0.0")
            .with_env_filter("debug");

        assert_eq!(config.global_fields.get("app").unwrap(), "test-app");
        assert_eq!(config.service_name.unwrap(), "test-service");
        assert_eq!(config.service_version.unwrap(), "1.0.0");
        assert_eq!(config.env_filter.unwrap(), "debug");
    }

    #[test]
    fn test_logging_context() {
        let context = LoggingContext::new("test-correlation-123".to_string())
            .with_request_id("req-456".to_string())
            .with_user_id("user-789".to_string())
            .with_custom_field("component", "test");

        let json = context.to_json();
        assert_eq!(json["correlation_id"], "test-correlation-123");
        assert_eq!(json["request_id"], "req-456");
        assert_eq!(json["user_id"], "user-789");
        assert_eq!(json["component"], "test");
    }

    #[test]
    fn test_structured_logging_utilities() {
        use structured::*;

        let context =
            LoggingContext::new("test-123".to_string()).with_user_id("user-456".to_string());

        // These would normally output to the configured logger
        // In tests, we just verify they don't panic
        log_http_request(&context, "GET", "/api/users", 200, 150, Some("test-agent"));
        log_database_query(&context, "SELECT * FROM users", 25, Some(5));
        log_application_error(
            &context,
            "ValidationError",
            "Invalid input",
            Some("Field 'email' is required"),
        );
        log_security_event(
            &context,
            "failed_login",
            "medium",
            "Multiple failed attempts",
            Some("192.168.1.100"),
        );
    }
}

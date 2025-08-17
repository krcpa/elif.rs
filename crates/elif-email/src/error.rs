use thiserror::Error;

/// Email system errors
#[derive(Error, Debug)]
pub enum EmailError {
    #[error("Template error: {message}")]
    Template { message: String },

    #[error("Provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("Queue error: {message}")]
    Queue { message: String },

    #[error("Tracking error: {message}")]
    Tracking { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("IO error: {message}")]
    Io { message: String },
}

impl EmailError {
    pub fn template(message: impl Into<String>) -> Self {
        Self::Template {
            message: message.into(),
        }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn queue(message: impl Into<String>) -> Self {
        Self::Queue {
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }
}

// Convert from common error types
impl From<handlebars::RenderError> for EmailError {
    fn from(err: handlebars::RenderError) -> Self {
        Self::template(err.to_string())
    }
}

impl From<handlebars::TemplateError> for EmailError {
    fn from(err: handlebars::TemplateError) -> Self {
        Self::template(err.to_string())
    }
}

impl From<lettre::error::Error> for EmailError {
    fn from(err: lettre::error::Error) -> Self {
        Self::provider("SMTP", err.to_string())
    }
}

impl From<lettre::transport::smtp::Error> for EmailError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        Self::provider("SMTP", err.to_string())
    }
}

impl From<reqwest::Error> for EmailError {
    fn from(err: reqwest::Error) -> Self {
        Self::network(err.to_string())
    }
}

impl From<serde_json::Error> for EmailError {
    fn from(err: serde_json::Error) -> Self {
        Self::validation("json", err.to_string())
    }
}

impl From<std::io::Error> for EmailError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}
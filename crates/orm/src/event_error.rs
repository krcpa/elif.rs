use std::fmt;

#[derive(Debug, Clone)]
pub enum EventError {
    Validation {
        message: String,
        hint: Option<String>,
    },
    Database {
        message: String,
    },
    Observer {
        message: String,
    },
    PropagationStopped {
        reason: String,
    },
}

impl EventError {
    pub fn validation(message: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
            hint: None,
        }
    }

    pub fn validation_with_hint(message: &str, hint: &str) -> Self {
        Self::Validation {
            message: message.to_string(),
            hint: Some(hint.to_string()),
        }
    }

    pub fn database(message: &str) -> Self {
        Self::Database {
            message: message.to_string(),
        }
    }

    pub fn observer(message: &str) -> Self {
        Self::Observer {
            message: message.to_string(),
        }
    }

    pub fn propagation_stopped(reason: &str) -> Self {
        Self::PropagationStopped {
            reason: reason.to_string(),
        }
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventError::Validation { message, hint } => {
                write!(f, "Validation error: {}", message)?;
                if let Some(hint) = hint {
                    write!(f, " (hint: {})", hint)?;
                }
                Ok(())
            }
            EventError::Database { message } => write!(f, "Database error: {}", message),
            EventError::Observer { message } => write!(f, "Observer error: {}", message),
            EventError::PropagationStopped { reason } => {
                write!(f, "Event propagation stopped: {}", reason)
            }
        }
    }
}

impl std::error::Error for EventError {}

impl From<std::io::Error> for EventError {
    fn from(err: std::io::Error) -> Self {
        Self::database(&err.to_string())
    }
}

impl From<sqlx::Error> for EventError {
    fn from(err: sqlx::Error) -> Self {
        Self::database(&err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_error_validation() {
        let error = EventError::validation("Invalid email format");

        match error {
            EventError::Validation { message, hint } => {
                assert_eq!(message, "Invalid email format");
                assert!(hint.is_none());
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_event_error_validation_with_hint() {
        let error =
            EventError::validation_with_hint("Invalid email format", "Use format user@domain.com");

        match error {
            EventError::Validation { message, hint } => {
                assert_eq!(message, "Invalid email format");
                assert_eq!(hint.unwrap(), "Use format user@domain.com");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_event_error_database() {
        let error = EventError::database("Connection timeout");

        match error {
            EventError::Database { message, .. } => {
                assert_eq!(message, "Connection timeout");
            }
            _ => panic!("Expected database error"),
        }
    }

    #[tokio::test]
    async fn test_event_error_observer() {
        let error = EventError::observer("Observer failed to execute");

        match error {
            EventError::Observer { message, .. } => {
                assert_eq!(message, "Observer failed to execute");
            }
            _ => panic!("Expected observer error"),
        }
    }

    #[tokio::test]
    async fn test_event_error_propagation_stopped() {
        let error = EventError::propagation_stopped("User cancelled operation");

        match error {
            EventError::PropagationStopped { reason, .. } => {
                assert_eq!(reason, "User cancelled operation");
            }
            _ => panic!("Expected propagation stopped error"),
        }
    }

    #[tokio::test]
    async fn test_event_error_display() {
        let error = EventError::validation("Test error");
        let display_message = format!("{}", error);
        assert!(display_message.contains("Test error"));
    }

    #[tokio::test]
    async fn test_event_error_debug() {
        let error = EventError::validation("Test error");
        let debug_message = format!("{:?}", error);
        assert!(debug_message.contains("Validation"));
        assert!(debug_message.contains("Test error"));
    }

    #[tokio::test]
    async fn test_event_error_conversion_from_std_error() {
        let std_error = std::io::Error::new(std::io::ErrorKind::Other, "IO error");
        let event_error: EventError = std_error.into();

        match event_error {
            EventError::Database { message, .. } => {
                assert!(message.contains("IO error"));
            }
            _ => panic!("Expected database error from std error conversion"),
        }
    }
}

//! Logging context utilities

use std::collections::HashMap;
use uuid::Uuid;

/// Logging context for request tracing
#[derive(Debug, Clone)]
pub struct LoggingContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub additional_fields: HashMap<String, String>,
}

impl LoggingContext {
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            user_id: None,
            session_id: None,
            additional_fields: HashMap::new(),
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn add_field(&mut self, key: String, value: String) {
        self.additional_fields.insert(key, value);
    }
}

impl Default for LoggingContext {
    fn default() -> Self {
        Self::new()
    }
}
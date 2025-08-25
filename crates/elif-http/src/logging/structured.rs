//! Structured logging utilities

use serde_json::{json, Value};

/// Create a structured log entry
pub fn log_entry(level: &str, message: &str, fields: Option<Value>) -> Value {
    let mut entry = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": level,
        "message": message
    });

    if let Some(Value::Object(fields_map)) = fields {
        if let Value::Object(entry_map) = &mut entry {
            entry_map.extend(fields_map);
        }
    }

    entry
}

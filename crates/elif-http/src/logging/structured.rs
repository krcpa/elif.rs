//! Structured logging utilities

pub mod structured {
    use serde_json::{json, Value};
    
    /// Create a structured log entry
    pub fn log_entry(level: &str, message: &str, fields: Option<Value>) -> Value {
        let mut entry = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": level,
            "message": message
        });
        
        if let Some(fields) = fields {
            if let Value::Object(ref mut map) = entry {
                if let Value::Object(fields_map) = fields {
                    for (key, value) in fields_map {
                        map.insert(key, value);
                    }
                }
            }
        }
        
        entry
    }
}
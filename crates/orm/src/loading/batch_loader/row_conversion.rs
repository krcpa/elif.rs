use serde_json::Value as JsonValue;

/// Row to JSON conversion functionality
impl super::BatchLoader {
    /// Convert a PostgreSQL row to JSON Value
    pub(super) fn row_to_json(&self, row: &sqlx::postgres::PgRow) -> Result<JsonValue, String> {
        use sqlx::{Row, Column};
        let mut map = serde_json::Map::new();
        
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            
            // Try to get the value as different PostgreSQL types
            let json_value = if let Ok(value) = row.try_get::<Option<String>, _>(i) {
                value.map_or(JsonValue::Null, JsonValue::String)
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::Number(serde_json::Number::from(v)))
            } else if let Ok(value) = row.try_get::<Option<i32>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::Number(serde_json::Number::from(v)))
            } else if let Ok(value) = row.try_get::<Option<f64>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::Number(
                    serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0))
                ))
            } else if let Ok(value) = row.try_get::<Option<f32>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::Number(
                    serde_json::Number::from_f64(v as f64).unwrap_or(serde_json::Number::from(0))
                ))
            } else if let Ok(value) = row.try_get::<Option<bool>, _>(i) {
                value.map_or(JsonValue::Null, JsonValue::Bool)
            } else if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::String(v.to_rfc3339()))
            } else if let Ok(value) = row.try_get::<Option<uuid::Uuid>, _>(i) {
                value.map_or(JsonValue::Null, |v| JsonValue::String(v.to_string()))
            } else if let Ok(value) = row.try_get::<Option<serde_json::Value>, _>(i) {
                value.unwrap_or(JsonValue::Null)
            } else {
                // Fallback: try to get as string or return null
                if let Ok(value) = row.try_get::<Option<String>, _>(i) {
                    value.map_or(JsonValue::Null, JsonValue::String)
                } else {
                    JsonValue::Null
                }
            };
            
            map.insert(column_name.to_string(), json_value);
        }
        
        Ok(JsonValue::Object(map))
    }
}
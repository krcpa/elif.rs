//! Minimal SQL Injection Prevention Tests

#[cfg(test)]
mod tests {
    use crate::security::{escape_identifier, validate_identifier, validate_parameter};

    #[test]
    fn test_identifier_escaping_basic() {
        assert_eq!(escape_identifier("users"), "\"users\"");
        assert_eq!(escape_identifier("table\"name"), "\"table\"\"name\"");
    }

    #[test]
    fn test_identifier_validation_basic() {
        assert!(validate_identifier("users").is_ok());
        assert!(validate_identifier("SELECT").is_err());
        assert!(validate_identifier("'; DROP TABLE users; --").is_err());
    }

    #[test]
    fn test_parameter_validation_basic() {
        assert!(validate_parameter("normal value").is_ok());
        // With escape-focused approach, these should be OK since they'll be parameterized
        assert!(validate_parameter("'; DROP TABLE users; --").is_ok());
        assert!(validate_parameter("value with \0 null byte").is_ok());
    }

    #[test]
    fn test_sql_generation_escapes_identifiers() {
        use crate::query::QueryBuilder;
        use serde_json::json;
        
        let (sql, _params) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .to_sql_with_params();
        
        // Table name should be escaped
        assert!(sql.contains("FROM \"users\""));
        assert!(sql.contains("SELECT *"));
    }

    #[test] 
    fn test_sql_generation_prevents_basic_injection() {
        use crate::query::QueryBuilder;
        use serde_json::json;
        
        let (sql, params) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("name", json!("'; DROP TABLE users; --"))
            .to_sql_with_params();
        
        // Malicious input should be parameterized
        assert!(sql.contains("$1"));
        assert_eq!(params[0], "'; DROP TABLE users; --");
        // Should not contain the raw malicious SQL
        assert!(!sql.contains("DROP TABLE users"));
        // Table name should be escaped
        assert!(sql.contains("FROM \"users\""));
    }
}
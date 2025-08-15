//! Focused SQL Injection Prevention Tests
//! 
//! This module contains tests to verify that the ORM prevents
//! SQL injection attacks through proper identifier escaping and 
//! parameter sanitization.

#[cfg(test)]
mod tests {
    use crate::security::{
        escape_identifier, validate_identifier, validate_parameter, 
        validate_query_pattern, IdentifierWhitelist
    };
    use crate::query::QueryBuilder;
    use serde_json::json;

    // ===== IDENTIFIER ESCAPING TESTS =====

    #[test]
    fn test_identifier_escaping_basic_cases() {
        // Normal identifiers should be wrapped in quotes
        assert_eq!(escape_identifier("users"), "\"users\"");
        assert_eq!(escape_identifier("user_table"), "\"user_table\"");
        assert_eq!(escape_identifier("table123"), "\"table123\"");
        assert_eq!(escape_identifier("_private"), "\"_private\"");
    }

    #[test]
    fn test_identifier_escaping_special_characters() {
        // Identifiers with quotes should be escaped by doubling
        assert_eq!(escape_identifier("table\"name"), "\"table\"\"name\"");
        assert_eq!(escape_identifier("user's_table"), "\"user's_table\"");
        assert_eq!(escape_identifier("col\"with\"quotes"), "\"col\"\"with\"\"quotes\"");
    }

    #[test]
    fn test_identifier_escaping_injection_attempts() {
        // SQL injection attempts should be escaped safely rather than rejected
        let malicious_inputs = vec![
            ("'; DROP TABLE users; --", "\"'; DROP TABLE users; --\""),
            ("table; DELETE FROM admin", "\"table; DELETE FROM admin\""),
            ("users UNION SELECT * FROM secrets", "\"users UNION SELECT * FROM secrets\""),
            ("table WHERE 1=1 OR 1=1", "\"table WHERE 1=1 OR 1=1\""),
            ("'; INSERT INTO log VALUES ('hacked'); --", "\"'; INSERT INTO log VALUES ('hacked'); --\"")
        ];

        for (input, expected) in malicious_inputs {
            let escaped = escape_identifier(input);
            // Should escape the content safely
            assert_eq!(escaped, expected);
        }
    }

    // ===== IDENTIFIER VALIDATION TESTS =====

    #[test]
    fn test_identifier_validation_valid_cases() {
        assert!(validate_identifier("users").is_ok());
        assert!(validate_identifier("user_table").is_ok());
        assert!(validate_identifier("table123").is_ok());
        assert!(validate_identifier("_private").is_ok());
        assert!(validate_identifier("my_table_name").is_ok());
    }

    #[test]
    fn test_identifier_validation_invalid_cases() {
        // Empty identifier
        assert!(validate_identifier("").is_err());
        
        // Starts with number
        assert!(validate_identifier("1table").is_err());
        
        // Contains invalid characters
        assert!(validate_identifier("table-name").is_err());
        assert!(validate_identifier("table name").is_err());
        assert!(validate_identifier("table@name").is_err());
        
        // SQL keywords
        assert!(validate_identifier("SELECT").is_err());
        assert!(validate_identifier("select").is_err());
        assert!(validate_identifier("INSERT").is_err());
        assert!(validate_identifier("UPDATE").is_err());
        assert!(validate_identifier("DELETE").is_err());
    }

    // ===== PARAMETER VALIDATION TESTS =====

    #[test]
    fn test_parameter_validation_safe_values() {
        assert!(validate_parameter("normal value").is_ok());
        assert!(validate_parameter("123").is_ok());
        assert!(validate_parameter("user@example.com").is_ok());
        assert!(validate_parameter("John's data").is_ok());
        assert!(validate_parameter("").is_ok()); // Empty string is OK
    }

    #[test]
    fn test_parameter_validation_size_limits() {
        // Should accept reasonably sized strings
        let medium_string = "a".repeat(1000);
        assert!(validate_parameter(&medium_string).is_ok());
        
        // Should reject extremely large strings
        let huge_string = "a".repeat(100000);
        assert!(validate_parameter(&huge_string).is_err());
    }

    // ===== QUERY PATTERN VALIDATION TESTS =====

    #[test]
    fn test_query_pattern_validation_safe_queries() {
        assert!(validate_query_pattern("SELECT * FROM users").is_ok());
        assert!(validate_query_pattern("INSERT INTO users VALUES ($1, $2)").is_ok());
        assert!(validate_query_pattern("UPDATE users SET name = $1 WHERE id = $2").is_ok());
        assert!(validate_query_pattern("DELETE FROM users WHERE id = $1").is_ok());
    }

    #[test]
    fn test_query_pattern_validation_dangerous_patterns() {
        assert!(validate_query_pattern("SELECT * FROM users; DROP TABLE users").is_err());
        assert!(validate_query_pattern("SELECT * FROM users UNION SELECT * FROM secrets").is_err());
        assert!(validate_query_pattern("EXEC sp_executesql 'SELECT * FROM users'").is_err());
        assert!(validate_query_pattern("SELECT * FROM users WHERE name = 'test'; --").is_err());
    }

    // ===== INTEGRATION TESTS =====

    #[test]
    fn test_select_query_identifier_escaping() {
        let (sql, params) = QueryBuilder::<()>::new()
            .select("name")
            .from("users")
            .where_eq("status", json!("active"))
            .to_sql_with_params();
        
        // Identifiers should be escaped
        assert!(sql.contains("FROM \"users\""));
        assert!(sql.contains("WHERE \"status\" = $1"));
        
        // Value should be parameterized
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "active");
    }

    #[test]
    fn test_insert_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .insert_into("users")
            .set("name", json!("John Doe"))
            .set("email", json!("john@example.com"))
            .to_sql_with_params();
        
        // Table and column names should be escaped
        assert!(sql.contains("INSERT INTO \"users\""));
        assert!(sql.contains("(\"name\", \"email\")"));
        
        // Values should be parameterized
        assert!(sql.contains("VALUES ($1, $2)"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "John Doe");
        assert_eq!(params[1], "john@example.com");
    }

    #[test]
    fn test_update_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .update("users")
            .set("name", json!("Jane Doe"))
            .where_eq("id", json!(1))
            .to_sql_with_params();
        
        // Table and column names should be escaped
        assert!(sql.contains("UPDATE \"users\""));
        assert!(sql.contains("SET \"name\" = $1"));
        assert!(sql.contains("WHERE \"id\" = $2"));
        
        // Values should be parameterized
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "Jane Doe");
        assert_eq!(params[1], "1");
    }

    #[test]
    fn test_delete_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .delete_from("users; DROP DATABASE production")
            .where_eq("name; INSERT INTO backdoor VALUES ('pwned')", json!("target"))
            .to_sql_with_params();
        
        // Should properly escape dangerous identifiers (they appear within quotes, so they're safe)
        // The dangerous SQL is neutralized by being part of escaped identifier names
        assert!(sql.contains("DELETE FROM \"users; DROP DATABASE production\""));
        assert!(sql.contains("WHERE \"name; INSERT INTO backdoor VALUES ('pwned')\" = $1"));
        
        // Value should be parameterized
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "target");
        
        // Should not contain unescaped/unquoted dangerous SQL
        assert!(!sql.contains("DROP DATABASE production;"));
        assert!(!sql.contains("INSERT INTO backdoor;"));
    }

    #[test]
    fn test_security_validation_method() {
        let builder = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("name", json!("test"));
            
        // Should pass security validation with normal queries
        assert!(builder.to_sql_with_params_secure().is_ok());
    }

    // ===== IDENTIFIER WHITELIST TESTS =====

    #[test]
    fn test_identifier_whitelist_basic_usage() {
        let whitelist = IdentifierWhitelist::new(vec!["users", "posts", "comments"]);
        
        assert!(whitelist.validate("users").is_ok());
        assert!(whitelist.validate("posts").is_ok());
        assert!(whitelist.validate("admin_table").is_err());
    }

    #[test]
    fn test_identifier_whitelist_escaping() {
        let whitelist = IdentifierWhitelist::new(vec!["users", "posts", "comments"]);
        
        assert_eq!(whitelist.escape_if_allowed("users").unwrap(), "\"users\"");
        assert!(whitelist.escape_if_allowed("hacker_table").is_err());
    }
}
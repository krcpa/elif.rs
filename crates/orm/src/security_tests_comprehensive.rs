//! Comprehensive SQL Injection Prevention Tests
//! 
//! This module contains detailed tests to verify that the ORM prevents
//! various types of SQL injection attacks through proper parameterization 
//! and identifier escaping.

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
        // SQL injection attempts should be handled safely
        let malicious_inputs = vec![
            "'; DROP TABLE users; --",
            "table; DELETE FROM admin",
            "users UNION SELECT * FROM secrets",
            "table WHERE 1=1 OR 1=1",
            "'; INSERT INTO log VALUES ('hacked'); --"
        ];

        for input in malicious_inputs {
            let escaped = escape_identifier(input);
            // Should return safe default for invalid identifiers
            assert_eq!(escaped, "\"invalid_identifier\"");
        }
    }

    // ===== IDENTIFIER VALIDATION TESTS =====

    #[test]
    fn test_identifier_validation_valid_cases() {
        let valid_identifiers = vec![
            "users", "user_table", "table123", "_private", "a", "Table_Name_123"
        ];

        for identifier in valid_identifiers {
            assert!(validate_identifier(identifier).is_ok(), 
                    "Should accept valid identifier: {}", identifier);
        }
    }

    #[test]
    fn test_identifier_validation_invalid_cases() {
        let invalid_cases = vec![
            ("", "empty identifier"),
            ("1table", "starts with number"),
            ("table-name", "contains hyphen"),
            ("table name", "contains space"),
            ("table.name", "contains dot"),
            ("table@name", "contains at symbol"),
            ("table#name", "contains hash"),
            ("table$name", "contains dollar (invalid in this context)"),
            ("SELECT", "SQL keyword uppercase"),
            ("select", "SQL keyword lowercase"),
            ("INSERT", "SQL keyword INSERT"),
            ("UPDATE", "SQL keyword UPDATE"),
            ("DELETE", "SQL keyword DELETE"),
            ("DROP", "SQL keyword DROP"),
            ("UNION", "SQL keyword UNION"),
        ];

        for (identifier, reason) in invalid_cases {
            assert!(validate_identifier(identifier).is_err(), 
                    "Should reject invalid identifier '{}' ({})", identifier, reason);
        }
    }

    #[test]
    fn test_identifier_validation_length_limits() {
        // Should accept identifiers up to 63 characters
        let max_length = "a".repeat(63);
        assert!(validate_identifier(&max_length).is_ok());

        // Should reject identifiers over 63 characters
        let too_long = "a".repeat(64);
        assert!(validate_identifier(&too_long).is_err());
    }

    #[test]
    fn test_identifier_validation_injection_patterns() {
        let injection_patterns = vec![
            "'; DROP TABLE users; --",
            "table; DELETE FROM admin",
            "users UNION SELECT password FROM secrets",
            "table' OR '1'='1",
            "admin'/**/OR/**/1=1--",
            "table WHERE 1=1; INSERT INTO log VALUES ('pwned')",
            "users'; EXEC xp_cmdshell('dir'); --"
        ];

        for pattern in injection_patterns {
            assert!(validate_identifier(pattern).is_err(),
                    "Should reject injection pattern: {}", pattern);
        }
    }

    // ===== PARAMETER VALIDATION TESTS =====

    #[test]
    fn test_parameter_validation_safe_values() {
        let safe_values = vec![
            "normal string",
            "user@example.com", 
            "123456",
            "John Doe",
            "Product Name",
            "Description with (parentheses)",
            "Text with numbers 123 and symbols !@#",
        ];

        for value in safe_values {
            assert!(validate_parameter(value).is_ok(),
                    "Should accept safe parameter: {}", value);
        }
    }

    #[test]
    fn test_parameter_validation_dangerous_patterns() {
        let dangerous_patterns = vec![
            // SQL injection attempts
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'/**/OR/**/1=1--",
            "1; DELETE FROM users; --",
            "' UNION SELECT password FROM admin --",
            
            // Command injection attempts  
            "'; EXEC xp_cmdshell('dir'); --",
            "1'; CALL dangerous_procedure(); --",
            
            // Blind injection attempts
            "1 AND (SELECT CASE WHEN 1=1 THEN pg_sleep(10) ELSE 0 END)",
            "1' AND SLEEP(5) AND 'a'='a",
            
            // Stacked queries
            "1; INSERT INTO log VALUES ('hacked')",
            "user'; UPDATE users SET role='admin' WHERE id=1; --"
        ];

        for pattern in dangerous_patterns {
            assert!(validate_parameter(pattern).is_err(),
                    "Should reject dangerous parameter: {}", pattern);
        }
    }

    #[test]
    fn test_parameter_validation_null_bytes() {
        let null_byte_patterns = vec![
            "value\x00with null",
            "admin\0password",
            "text\x00\x00multiple nulls"
        ];

        for pattern in null_byte_patterns {
            assert!(validate_parameter(pattern).is_err(),
                    "Should reject parameter with null bytes: {:?}", pattern);
        }
    }

    #[test]
    fn test_parameter_validation_size_limits() {
        // Should accept parameters up to 64KB
        let large_param = "a".repeat(65536);
        assert!(validate_parameter(&large_param).is_ok());

        // Should reject parameters over 64KB
        let too_large = "a".repeat(65537);
        assert!(validate_parameter(&too_large).is_err());
    }

    // ===== QUERY PATTERN VALIDATION TESTS =====

    #[test]
    fn test_query_pattern_validation_safe_queries() {
        let safe_queries = vec![
            "SELECT * FROM users",
            "SELECT id, name FROM users WHERE active = $1",
            "INSERT INTO users (name, email) VALUES ($1, $2)",
            "UPDATE users SET name = $1 WHERE id = $2", 
            "DELETE FROM users WHERE id = $1",
            "SELECT COUNT(*) FROM posts WHERE user_id = $1"
        ];

        for query in safe_queries {
            assert!(validate_query_pattern(query).is_ok(),
                    "Should accept safe query: {}", query);
        }
    }

    #[test]
    fn test_query_pattern_validation_dangerous_patterns() {
        let dangerous_queries = vec![
            // Multiple statements
            "SELECT * FROM users; DROP TABLE users",
            "INSERT INTO users VALUES ($1); DELETE FROM admin",
            
            // System function calls
            "EXEC xp_cmdshell 'dir'",
            "EXECUTE sp_configure 'show advanced options', 1",
            "SELECT * FROM OPENROWSET('Microsoft.ACE.OLEDB.12.0')",
            
            // Union attacks  
            "SELECT * FROM users UNION SELECT * FROM secrets",
            "SELECT name FROM users UNION ALL SELECT password FROM admin",
            
            // Information schema access
            "SELECT * FROM INFORMATION_SCHEMA.TABLES",
            "SELECT * FROM SYS.TABLES",
            "SELECT * FROM SYSOBJECTS",
            
            // Comment-based attacks
            "SELECT * FROM users WHERE id = 1'; --",
            "SELECT * FROM users /*comment*/ WHERE 1=1",
            
            // System stored procedures
            "EXEC SP_CONFIGURE",
            "EXEC XP_CMDSHELL",
        ];

        for query in dangerous_queries {
            assert!(validate_query_pattern(query).is_err(),
                    "Should reject dangerous query: {}", query);
        }
    }

    // ===== IDENTIFIER WHITELIST TESTS =====

    #[test]
    fn test_identifier_whitelist_basic_usage() {
        let whitelist = IdentifierWhitelist::new(vec!["users", "posts", "comments"]);
        
        // Should accept whitelisted identifiers
        assert!(whitelist.validate("users").is_ok());
        assert!(whitelist.validate("posts").is_ok());
        assert!(whitelist.validate("comments").is_ok());
        
        // Should reject non-whitelisted identifiers
        assert!(whitelist.validate("admin").is_err());
        assert!(whitelist.validate("secrets").is_err());
        assert!(whitelist.validate("'; DROP TABLE users; --").is_err());
    }

    #[test]
    fn test_identifier_whitelist_escaping() {
        let whitelist = IdentifierWhitelist::new(vec!["user_table", "post_data"]);
        
        // Should escape allowed identifiers
        assert_eq!(whitelist.escape_if_allowed("user_table").unwrap(), "\"user_table\"");
        assert_eq!(whitelist.escape_if_allowed("post_data").unwrap(), "\"post_data\"");
        
        // Should reject disallowed identifiers
        assert!(whitelist.escape_if_allowed("malicious_table").is_err());
    }

    // ===== SQL GENERATION SECURITY TESTS =====

    #[test]
    fn test_select_query_identifier_escaping() {
        let (sql, _) = QueryBuilder::<()>::new()
            .select("id, name, email")
            .from("users")
            .to_sql_with_params();
        
        // All identifiers should be escaped
        assert!(sql.contains("SELECT \"id\", \"name\", \"email\""));
        assert!(sql.contains("FROM \"users\""));
    }

    #[test]
    fn test_select_query_with_malicious_table_name() {
        let (sql, _) = QueryBuilder::<()>::new()
            .select("*")
            .from("users; DROP TABLE admin; --")
            .to_sql_with_params();
        
        // Malicious table name should be escaped
        assert!(sql.contains("FROM \"users; DROP TABLE admin; --\""));
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("DROP TABLE admin"));
    }

    #[test]
    fn test_where_clause_parameter_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("name", json!("'; DROP TABLE users; --"))
            .to_sql_with_params();
        
        // Malicious value should be parameterized
        assert!(sql.contains("WHERE \"name\" = $1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "'; DROP TABLE users; --");
        
        // Should not contain unescaped SQL injection
        assert!(!sql.contains("DROP TABLE users"));
    }

    #[test]
    fn test_join_identifier_escaping() {
        let (sql, _) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .join("posts; DELETE FROM admin", "users.id", "posts.user_id")
            .to_sql_with_params();
        
        // Join table and column names should be escaped
        assert!(sql.contains("JOIN \"posts; DELETE FROM admin\""));
        assert!(sql.contains("\"users.id\" = \"posts.user_id\""));
        
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("DELETE FROM admin"));
    }

    #[test]
    fn test_insert_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .insert_into("users; DROP TABLE secrets")
            .set("name; UPDATE admin SET password = 'hacked'", json!("John"))
            .set("email", json!("john@example.com"))
            .to_sql_with_params();
        
        // Table and column names should be escaped
        assert!(sql.contains("INSERT INTO \"users; DROP TABLE secrets\""));
        assert!(sql.contains("\"name; UPDATE admin SET password = 'hacked'\""));
        assert!(sql.contains("\"email\""));
        
        // Values should be parameterized
        assert!(sql.contains("$1"));
        assert!(sql.contains("$2"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "John");
        assert_eq!(params[1], "john@example.com");
        
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("DROP TABLE secrets"));
        assert!(!sql.contains("UPDATE admin"));
    }

    #[test]
    fn test_update_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .update("users; INSERT INTO log VALUES ('hacked')")
            .set("name; DROP TABLE admin", json!("'; DELETE FROM users; --"))
            .where_eq("id; TRUNCATE secrets", json!(1))
            .to_sql_with_params();
        
        // All identifiers should be escaped
        assert!(sql.contains("UPDATE \"users; INSERT INTO log VALUES ('hacked')\""));
        assert!(sql.contains("SET \"name; DROP TABLE admin\" = $1"));
        assert!(sql.contains("WHERE \"id; TRUNCATE secrets\" = $2"));
        
        // Values should be parameterized
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "'; DELETE FROM users; --");
        assert_eq!(params[1], "1");
        
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("INSERT INTO log"));
        assert!(!sql.contains("DROP TABLE admin"));
        assert!(!sql.contains("TRUNCATE secrets"));
        assert!(!sql.contains("DELETE FROM users"));
    }

    #[test]
    fn test_delete_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .delete_from("users; DROP DATABASE production")
            .where_eq("name; INSERT INTO backdoor VALUES ('pwned')", json!("target"))
            .to_sql_with_params();
        
        // Identifiers should be escaped
        assert!(sql.contains("DELETE FROM \"users; DROP DATABASE production\""));
        assert!(sql.contains("WHERE \"name; INSERT INTO backdoor VALUES ('pwned')\" = $1"));
        
        // Value should be parameterized
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "target");
        
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("DROP DATABASE production"));
        assert!(!sql.contains("INSERT INTO backdoor"));
    }

    #[test]
    fn test_order_by_safety() {
        let (sql, _) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .order_by("name; DELETE FROM admin; --")
            .to_sql_with_params();
        
        // Order by column should be escaped
        assert!(sql.contains("ORDER BY \"name; DELETE FROM admin; --\" ASC"));
        
        // Should not contain unescaped malicious SQL
        assert!(!sql.contains("DELETE FROM admin"));
    }

    #[test]
    fn test_complex_multi_table_query_safety() {
        let (sql, params) = QueryBuilder::<()>::new()
            .select("u.name, p.title")
            .from("users; DROP TABLE secrets")
            .join("posts; UPDATE admin SET role = 'hacker'", "users.id", "posts.user_id")
            .where_eq("u.status; DELETE FROM logs", json!("active"))
            .where_eq("p.published; EXEC xp_cmdshell('rm -rf /')", json!(true))
            .order_by_desc("u.created_at; INSERT INTO backdoor VALUES ('owned')")
            .limit(10)
            .to_sql_with_params();
        
        // All identifiers should be properly escaped
        assert!(sql.contains("SELECT \"u.name\", \"p.title\""));
        assert!(sql.contains("FROM \"users; DROP TABLE secrets\""));
        assert!(sql.contains("JOIN \"posts; UPDATE admin SET role = 'hacker'\""));
        assert!(sql.contains("WHERE \"u.status; DELETE FROM logs\" = $1"));
        assert!(sql.contains("AND \"p.published; EXEC xp_cmdshell('rm -rf /')\" = $2"));
        assert!(sql.contains("ORDER BY \"u.created_at; INSERT INTO backdoor VALUES ('owned')\" DESC"));
        assert!(sql.contains("LIMIT 10"));
        
        // Parameters should be properly bound
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], "active");
        assert_eq!(params[1], "true");
        
        // Should not contain any unescaped malicious SQL
        let malicious_patterns = vec![
            "DROP TABLE secrets",
            "UPDATE admin",
            "DELETE FROM logs", 
            "EXEC xp_cmdshell",
            "INSERT INTO backdoor"
        ];
        
        for pattern in malicious_patterns {
            assert!(!sql.contains(pattern), "SQL should not contain unescaped: {}", pattern);
        }
    }

    #[test]
    fn test_security_validation_method() {
        // Test that the secure validation method works
        let builder = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("name", json!("John"));
        
        // Normal query should pass validation
        let result = builder.to_sql_with_params_secure();
        assert!(result.is_ok());
        
        // Query with invalid identifier should fail validation
        let malicious_builder = QueryBuilder::<()>::new()
            .select("*")
            .from("'; DROP TABLE users; --");
        
        let result = malicious_builder.to_sql_with_params_secure();
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_cases_and_corner_cases() {
        // Test empty strings
        let (sql, _) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("", json!(""))
            .to_sql_with_params();
        
        // Empty column name should still be escaped (though it will be invalid)
        assert!(sql.contains("WHERE \"\" = $1"));
        
        // Test very long malicious input
        let long_malicious = "'; ".to_string() + &"DROP TABLE users; ".repeat(1000) + "--";
        let (sql, params) = QueryBuilder::<()>::new()
            .select("*")
            .from("users")
            .where_eq("id", json!(long_malicious.clone()))
            .to_sql_with_params();
        
        // Should still be parameterized regardless of length
        assert!(sql.contains("$1"));
        assert_eq!(params[0], long_malicious);
        assert!(!sql.contains("DROP TABLE users"));
    }

    #[test]
    fn test_unicode_and_encoding_edge_cases() {
        // Test Unicode characters that might confuse parsers
        let unicode_tests = vec![
            "admin\u{0000}' OR 1=1--",  // null byte
            "user\u{00A0}' OR 1=1--",   // non-breaking space
            "test\u{FEFF}' OR 1=1--",   // byte order mark
            "name\u{200B}' OR 1=1--",   // zero-width space
        ];
        
        for test_input in unicode_tests {
            let (sql, params) = QueryBuilder::<()>::new()
                .select("*")
                .from("users")
                .where_eq("name", json!(test_input))
                .to_sql_with_params();
            
            // Should be parameterized
            assert!(sql.contains("$1"));
            assert_eq!(params[0], test_input);
            assert!(!sql.contains("OR 1=1"));
            
            // Should fail security validation due to dangerous patterns
            let builder = QueryBuilder::<()>::new()
                .select("*")
                .from("users") 
                .where_eq("name", json!(test_input));
                
            assert!(builder.to_sql_with_params_secure().is_err());
        }
    }

    #[test]
    fn test_legitimate_queries_still_work() {
        // Comprehensive test of normal, legitimate usage
        let (sql, params) = QueryBuilder::<()>::new()
            .select("u.id, u.name, u.email, p.title, COUNT(c.id) as comment_count")
            .from("users")
            .join("posts", "users.id", "posts.user_id")
            .join("comments", "posts.id", "comments.post_id")
            .where_eq("u.active", json!(true))
            .where_like("u.name", "John%")
            .where_eq("p.published", json!(true))
            .order_by_desc("u.created_at")
            .order_by("p.title")
            .limit(25)
            .to_sql_with_params();
        
        // Verify proper SQL structure with escaping
        assert!(sql.contains("SELECT \"u.id\", \"u.name\", \"u.email\", \"p.title\", \"COUNT(c.id) as comment_count\""));
        assert!(sql.contains("FROM \"users\""));
        assert!(sql.contains("JOIN \"posts\" ON \"users.id\" = \"posts.user_id\""));
        assert!(sql.contains("JOIN \"comments\" ON \"posts.id\" = \"comments.post_id\""));
        assert!(sql.contains("WHERE \"u.active\" = $1"));
        assert!(sql.contains("AND \"u.name\" LIKE $2"));
        assert!(sql.contains("AND \"p.published\" = $3"));
        assert!(sql.contains("ORDER BY \"u.created_at\" DESC"));
        assert!(sql.contains("\"p.title\" ASC"));
        assert!(sql.contains("LIMIT 25"));
        
        // Verify parameters
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], "true");
        assert_eq!(params[1], "John%");
        assert_eq!(params[2], "true");
        
        // Should pass security validation
        let builder = QueryBuilder::<()>::new()
            .select("u.id, u.name, u.email")
            .from("users")
            .join("posts", "users.id", "posts.user_id")  
            .where_eq("u.active", json!(true))
            .order_by_desc("u.created_at")
            .limit(10);
            
        assert!(builder.to_sql_with_params_secure().is_ok());
    }
}
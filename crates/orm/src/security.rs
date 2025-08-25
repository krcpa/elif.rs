//! Security utilities for SQL injection prevention
//!
//! This module provides functions for:
//! - Escaping SQL identifiers (table names, column names)
//! - Validating identifier names
//! - Query pattern validation

use crate::error::ModelError;
use std::collections::HashSet;

/// Characters allowed in SQL identifiers (alphanumeric, underscore, dollar)
const ALLOWED_IDENTIFIER_CHARS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_$";

/// SQL keywords that must be escaped or rejected
static SQL_KEYWORDS: &[&str] = &[
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "FROM",
    "WHERE",
    "JOIN",
    "UNION",
    "DROP",
    "CREATE",
    "ALTER",
    "GRANT",
    "REVOKE",
    "TRUNCATE",
    "EXEC",
    "EXECUTE",
    "DECLARE",
    "CAST",
    "CONVERT",
    "SUBSTRING",
    "ASCII",
    "CHAR",
    "NCHAR",
    "SYSTEM",
    "USER",
    "SESSION_USER",
    "CURRENT_USER",
    "SUSER_NAME",
    "IS_MEMBER",
];

/// Escape a SQL identifier (table name, column name, etc.)
///
/// This function:
/// 1. Escapes any existing double quotes by doubling them
/// 2. Wraps the identifier in double quotes for safe SQL usage
///
/// This approach prioritizes escaping over validation - any identifier can be escaped safely.
///
/// # Arguments
/// * `identifier` - The identifier to escape
///
/// # Returns
/// * Escaped identifier safe for use in SQL
///
/// # Examples
/// ```
/// use elif_orm::security::escape_identifier;
///
/// assert_eq!(escape_identifier("user_table"), "\"user_table\"");
/// assert_eq!(escape_identifier("table\"name"), "\"table\"\"name\"");
/// ```
pub fn escape_identifier(identifier: &str) -> String {
    // Escape double quotes by doubling them
    let escaped = identifier.replace('\"', "\"\"");

    // Wrap in double quotes for PostgreSQL identifier escaping
    format!("\"{}\"", escaped)
}

/// Validate that an identifier is safe for use in SQL
///
/// # Arguments
/// * `identifier` - The identifier to validate
///
/// # Returns
/// * Ok(()) if valid, Err(ModelError) if invalid
pub fn validate_identifier(identifier: &str) -> Result<(), ModelError> {
    // Check for empty identifier
    if identifier.is_empty() {
        return Err(ModelError::Validation(
            "Identifier cannot be empty".to_string(),
        ));
    }

    // Check length (PostgreSQL limit is 63 characters)
    if identifier.len() > 63 {
        return Err(ModelError::Validation(format!(
            "Identifier '{}' is too long (max 63 characters)",
            identifier
        )));
    }

    // Check for allowed characters only
    for c in identifier.chars() {
        if !ALLOWED_IDENTIFIER_CHARS.contains(c) {
            return Err(ModelError::Validation(format!(
                "Identifier '{}' contains invalid character '{}'",
                identifier, c
            )));
        }
    }

    // Check that it doesn't start with a number
    if identifier.chars().next().unwrap().is_ascii_digit() {
        return Err(ModelError::Validation(format!(
            "Identifier '{}' cannot start with a number",
            identifier
        )));
    }

    // Check against SQL keywords (case insensitive)
    let upper_identifier = identifier.to_uppercase();
    if SQL_KEYWORDS.contains(&upper_identifier.as_str()) {
        return Err(ModelError::Validation(format!(
            "Identifier '{}' is a reserved SQL keyword",
            identifier
        )));
    }

    Ok(())
}

/// Validate query pattern to prevent dangerous SQL constructs
///
/// # Arguments
/// * `sql` - The SQL query to validate
///
/// # Returns
/// * Ok(()) if safe, Err(ModelError) if potentially dangerous
pub fn validate_query_pattern(sql: &str) -> Result<(), ModelError> {
    let sql_upper = sql.to_uppercase();

    // Check for multiple statements (semicolon not at the end)
    let semicolon_positions: Vec<_> = sql.match_indices(';').collect();
    if semicolon_positions.len() > 1
        || (semicolon_positions.len() == 1 && semicolon_positions[0].0 != sql.trim().len() - 1)
    {
        return Err(ModelError::Validation(
            "Multiple SQL statements not allowed".to_string(),
        ));
    }

    // Check for dangerous patterns
    let dangerous_patterns = [
        "EXEC ",
        "EXECUTE ",
        "SP_",
        "XP_",
        "OPENROWSET",
        "OPENDATASOURCE",
        "BULK INSERT",
        "BCP ",
        "SQLCMD",
        "OSQL",
        "ISQL",
        "UNION ALL SELECT",
        "UNION SELECT",
        "'; --",
        "'/*",
        "*/'",
        "INFORMATION_SCHEMA",
        "SYS.",
        "SYSOBJECTS",
        "SYSCOLUMNS",
    ];

    for pattern in &dangerous_patterns {
        if sql_upper.contains(pattern) {
            return Err(ModelError::Validation(format!(
                "Query contains potentially dangerous pattern: {}",
                pattern
            )));
        }
    }

    Ok(())
}

/// Validate parameter value to prevent injection through parameters
///
/// With the escape-focused approach, parameter validation is minimal since
/// parameters are properly parameterized and escaped by the database driver.
///
/// # Arguments
/// * `value` - The parameter value to validate
///
/// # Returns
/// * Ok(()) if safe, Err(ModelError) if potentially dangerous
pub fn validate_parameter(value: &str) -> Result<(), ModelError> {
    // Check for extremely long parameters (potential DoS)
    if value.len() > 65536 {
        // 64KB limit
        return Err(ModelError::Validation(
            "Parameter value too large (max 64KB)".to_string(),
        ));
    }

    // With proper parameterization, most content is safe
    // Only reject if there are genuine protocol-level risks

    Ok(())
}

/// Create a whitelist-based identifier validator
///
/// This creates a validator that only allows specific identifiers from a predefined list.
/// Useful for table names and column names that should be strictly controlled.
pub struct IdentifierWhitelist {
    allowed: HashSet<String>,
}

impl IdentifierWhitelist {
    /// Create a new whitelist validator
    pub fn new(allowed_identifiers: Vec<&str>) -> Self {
        let allowed = allowed_identifiers
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Self { allowed }
    }

    /// Validate that an identifier is in the whitelist
    pub fn validate(&self, identifier: &str) -> Result<(), ModelError> {
        if self.allowed.contains(identifier) {
            Ok(())
        } else {
            Err(ModelError::Validation(format!(
                "Identifier '{}' is not in the allowed whitelist",
                identifier
            )))
        }
    }

    /// Get escaped identifier if it's in the whitelist
    pub fn escape_if_allowed(&self, identifier: &str) -> Result<String, ModelError> {
        self.validate(identifier)?;
        Ok(escape_identifier(identifier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_identifier() {
        assert_eq!(escape_identifier("user_table"), "\"user_table\"");
        assert_eq!(escape_identifier("table\"name"), "\"table\"\"name\"");
        assert_eq!(escape_identifier("simple"), "\"simple\"");
    }

    #[test]
    fn test_validate_identifier() {
        assert!(validate_identifier("user_table").is_ok());
        assert!(validate_identifier("table1").is_ok());
        assert!(validate_identifier("_private").is_ok());

        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("1table").is_err());
        assert!(validate_identifier("table-name").is_err());
        assert!(validate_identifier("table name").is_err());
        assert!(validate_identifier("SELECT").is_err());
        assert!(validate_identifier("select").is_err());
    }

    #[test]
    fn test_validate_query_pattern() {
        assert!(validate_query_pattern("SELECT * FROM users").is_ok());
        assert!(validate_query_pattern("INSERT INTO users VALUES ($1, $2)").is_ok());

        assert!(validate_query_pattern("SELECT * FROM users; DROP TABLE users").is_err());
        assert!(validate_query_pattern("SELECT * FROM users UNION SELECT * FROM secrets").is_err());
        assert!(validate_query_pattern("EXEC sp_executesql 'SELECT * FROM users'").is_err());
    }

    #[test]
    fn test_validate_parameter() {
        assert!(validate_parameter("normal value").is_ok());
        assert!(validate_parameter("123").is_ok());
        assert!(validate_parameter("user@example.com").is_ok());
        // Parameters with SQL-like content are OK since they'll be parameterized
        assert!(validate_parameter("'; DROP TABLE users; --").is_ok());
        assert!(validate_parameter("UNION SELECT").is_ok());

        // With escape-focused approach, null bytes are also OK
        assert!(validate_parameter("value with \0 null byte").is_ok());
    }

    #[test]
    fn test_identifier_whitelist() {
        let whitelist = IdentifierWhitelist::new(vec!["users", "posts", "comments"]);

        assert!(whitelist.validate("users").is_ok());
        assert!(whitelist.validate("posts").is_ok());
        assert!(whitelist.validate("admin_table").is_err());

        assert_eq!(whitelist.escape_if_allowed("users").unwrap(), "\"users\"");
        assert!(whitelist.escape_if_allowed("hacker_table").is_err());
    }
}

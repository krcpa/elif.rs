//! SQL Generation and Security
//!
//! This module handles secure SQL generation with proper parameterization,
//! identifier validation, and dialect-specific SQL generation.

pub mod generation;

// Re-export for convenience
pub use generation::*;

/// SQL parameter placeholder generation for different database dialects
pub fn parameter_placeholder(index: usize, dialect: &crate::backends::SqlDialect) -> String {
    dialect.parameter_placeholder(index)
}

/// Escape SQL identifier for safe use in queries
pub fn escape_identifier(identifier: &str, dialect: &crate::backends::SqlDialect) -> String {
    let quote_char = dialect.identifier_quote();
    format!("{}{}{}", quote_char, identifier.replace(&quote_char.to_string(), &format!("{}{}", quote_char, quote_char)), quote_char)
}
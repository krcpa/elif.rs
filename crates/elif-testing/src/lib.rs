//! # elif-testing - Comprehensive Testing Framework
//!
//! A powerful testing framework designed specifically for elif.rs applications,
//! providing utilities for database testing, HTTP testing, factory-based test
//! data generation, and seamless integration with standard Rust test runners.
//!
//! ## Features
//!
//! - **Database Testing**: Automatic test database management with transactions
//! - **HTTP Testing**: Fluent test client with comprehensive assertions
//! - **Factory System**: Type-safe test data generation with relationships
//! - **Authentication Testing**: Built-in support for JWT and session testing
//! - **Performance Testing**: Load testing and benchmarking utilities
//! - **Rust Integration**: Seamless integration with `cargo test`
//!
//! ## Quick Start
//!
//! ```rust
//! use elif_testing::prelude::*;
//!
//! #[test]
//! fn test_utilities() {
//!     // Generate test data
//!     let test_email = utils::random_email();
//!     let test_id = utils::test_uuid();
//!     let timestamp = utils::test_timestamp();
//!     
//!     // Use utilities
//!     assert!(test_email.contains("@example.com"));
//!     assert!(!test_id.to_string().is_empty());
//!     assert!(!timestamp.to_rfc3339().is_empty());
//! }
//! ```

pub mod assertions;
pub mod auth;
pub mod client;
pub mod database;
pub mod factories;
pub mod performance;

// Re-export commonly used types
pub use assertions::TestAssertions;
pub use client::{TestClient, TestResponse};
pub use database::{DatabaseTransaction, TestDatabase};
pub use factories::{Factory, FactoryBuilder};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        assertions::TestAssertions,
        client::{TestClient, TestResponse},
        database::{DatabaseTransaction, TestDatabase},
        factories::{Factory, FactoryBuilder},
        utils,
    };

    // Re-export commonly used external types
    pub use chrono::{DateTime, Utc};
    pub use serde_json::{json, Value as JsonValue};
    pub use uuid::Uuid;
}

// Error handling
#[derive(thiserror::Error, Debug)]
pub enum TestError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Factory error: {message}")]
    Factory { message: String },

    #[error("Assertion failed: {message}")]
    Assertion { message: String },

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Test setup error: {0}")]
    Setup(String),
}

pub type TestResult<T> = Result<T, TestError>;

/// Test utilities and helper functions
pub mod utils {

    /// Generate a random test string with optional prefix
    pub fn random_string(prefix: Option<&str>) -> String {
        use rand::Rng;
        let suffix: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        match prefix {
            Some(p) => format!("{}_{}", p, suffix),
            None => suffix,
        }
    }

    /// Generate a random test email
    pub fn random_email() -> String {
        format!("test_{}@example.com", random_string(None).to_lowercase())
    }

    /// Create a test UUID
    pub fn test_uuid() -> uuid::Uuid {
        uuid::Uuid::new_v4()
    }

    /// Create a test timestamp
    pub fn test_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;

    #[test]
    fn test_random_string_generation() {
        let s1 = utils::random_string(None);
        let s2 = utils::random_string(None);

        assert_eq!(s1.len(), 8);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_random_string_with_prefix() {
        let s = utils::random_string(Some("test"));
        assert!(s.starts_with("test_"));
        assert!(s.len() > 5);
    }

    #[test]
    fn test_random_email_format() {
        let email = utils::random_email();
        assert!(email.contains("@example.com"));
        assert!(email.starts_with("test_"));
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = utils::test_uuid();
        let uuid2 = utils::test_uuid();
        assert_ne!(uuid1, uuid2);
    }
}

//! Database Backend Abstractions
//!
//! This module provides database backend abstractions to support multiple database types
//! (PostgreSQL, MySQL, SQLite, etc.) through common traits and interfaces.

pub mod core;
pub mod postgres;

// Re-export core traits and types
pub use core::*;
pub use postgres::PostgresBackend;

/// Database backend type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DatabaseBackendType {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl std::fmt::Display for DatabaseBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseBackendType::PostgreSQL => write!(f, "postgresql"),
            DatabaseBackendType::MySQL => write!(f, "mysql"),
            DatabaseBackendType::SQLite => write!(f, "sqlite"),
        }
    }
}

impl std::str::FromStr for DatabaseBackendType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" => Ok(DatabaseBackendType::PostgreSQL),
            "mysql" => Ok(DatabaseBackendType::MySQL),
            "sqlite" => Ok(DatabaseBackendType::SQLite),
            _ => Err(format!("Unsupported database backend: {}", s)),
        }
    }
}

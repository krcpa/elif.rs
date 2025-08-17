//! Transaction Isolation Levels
//!
//! Provides isolation level management for different database systems.

/// Transaction isolation levels supported by PostgreSQL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read Uncommitted - lowest isolation level
    ReadUncommitted,
    /// Read Committed - default PostgreSQL isolation level
    ReadCommitted,
    /// Repeatable Read - stronger consistency guarantees
    RepeatableRead,
    /// Serializable - highest isolation level
    Serializable,
}

impl IsolationLevel {
    /// Convert to SQL string for SET TRANSACTION ISOLATION LEVEL command
    pub fn as_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED", 
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
    
    /// Get the default isolation level for a database dialect
    pub fn default_for_dialect(dialect: &crate::backends::SqlDialect) -> Self {
        match dialect {
            crate::backends::SqlDialect::PostgreSQL => IsolationLevel::ReadCommitted,
            crate::backends::SqlDialect::MySQL => IsolationLevel::RepeatableRead,
            crate::backends::SqlDialect::SQLite => IsolationLevel::Serializable,
        }
    }
    
    /// Check if this isolation level is supported by the given dialect
    pub fn is_supported_by(&self, dialect: &crate::backends::SqlDialect) -> bool {
        match dialect {
            crate::backends::SqlDialect::PostgreSQL => true, // All levels supported
            crate::backends::SqlDialect::MySQL => {
                matches!(self, IsolationLevel::ReadUncommitted | IsolationLevel::ReadCommitted | 
                              IsolationLevel::RepeatableRead | IsolationLevel::Serializable)
            },
            crate::backends::SqlDialect::SQLite => {
                // SQLite only effectively supports Serializable
                matches!(self, IsolationLevel::Serializable)
            },
        }
    }
}
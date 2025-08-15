//! Error types for the ORM system
//!
//! Provides comprehensive error handling for database operations,
//! model validation, and query building.

use std::fmt;

/// Result type alias for model operations
pub type ModelResult<T> = Result<T, ModelError>;

/// ORM error type alias
pub type OrmError = ModelError;

/// ORM result type alias  
pub type OrmResult<T> = ModelResult<T>;

/// Error types for ORM operations
#[derive(Debug, Clone)]
pub enum ModelError {
    /// Database connection or query error
    Database(String),
    /// Model not found in database
    NotFound(String),
    /// Model validation failed
    Validation(String),
    /// Primary key is missing or invalid
    MissingPrimaryKey,
    /// Relationship loading failed
    Relationship(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Migration error
    Migration(String),
    /// Connection pool error
    Connection(String),
    /// Transaction error
    Transaction(String),
    /// Schema error
    Schema(String),
    /// Query building error
    Query(String),
    /// Event system error
    Event(String),
    /// Configuration error
    Configuration(String),
    /// Invalid key error
    InvalidKey(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Database(msg) => write!(f, "Database error: {}", msg),
            ModelError::NotFound(table) => write!(f, "Record not found in table '{}'", table),
            ModelError::Validation(msg) => write!(f, "Validation error: {}", msg),
            ModelError::MissingPrimaryKey => write!(f, "Primary key is missing or invalid"),
            ModelError::Relationship(msg) => write!(f, "Relationship error: {}", msg),
            ModelError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            ModelError::Migration(msg) => write!(f, "Migration error: {}", msg),
            ModelError::Connection(msg) => write!(f, "Connection error: {}", msg),
            ModelError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            ModelError::Schema(msg) => write!(f, "Schema error: {}", msg),
            ModelError::Query(msg) => write!(f, "Query error: {}", msg),
            ModelError::Event(msg) => write!(f, "Event error: {}", msg),
            ModelError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            ModelError::InvalidKey(msg) => write!(f, "Invalid key error: {}", msg),
        }
    }
}

impl std::error::Error for ModelError {}

// Convert from sqlx errors
impl From<sqlx::Error> for ModelError {
    fn from(err: sqlx::Error) -> Self {
        ModelError::Database(err.to_string())
    }
}

// Convert from serde_json errors
impl From<serde_json::Error> for ModelError {
    fn from(err: serde_json::Error) -> Self {
        ModelError::Serialization(err.to_string())
    }
}

// Convert from anyhow errors
impl From<anyhow::Error> for ModelError {
    fn from(err: anyhow::Error) -> Self {
        ModelError::Database(err.to_string())
    }
}

/// Error types for query builder operations
#[derive(Debug, Clone)]
pub enum QueryError {
    /// Invalid SQL syntax
    InvalidSql(String),
    /// Missing required fields
    MissingFields(String),
    /// Invalid parameter binding
    InvalidParameter(String),
    /// Unsupported operation
    UnsupportedOperation(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::InvalidSql(msg) => write!(f, "Invalid SQL: {}", msg),
            QueryError::MissingFields(msg) => write!(f, "Missing fields: {}", msg),
            QueryError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            QueryError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<QueryError> for ModelError {
    fn from(err: QueryError) -> Self {
        ModelError::Query(err.to_string())
    }
}

/// Error types for relationship operations
#[derive(Debug, Clone)]
pub enum RelationshipError {
    /// Relationship not found
    NotFound(String),
    /// Invalid relationship configuration
    InvalidConfiguration(String),
    /// Circular dependency in relationships
    CircularDependency(String),
    /// Foreign key constraint violation
    ForeignKeyViolation(String),
}

impl fmt::Display for RelationshipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationshipError::NotFound(msg) => write!(f, "Relationship not found: {}", msg),
            RelationshipError::InvalidConfiguration(msg) => write!(f, "Invalid relationship configuration: {}", msg),
            RelationshipError::CircularDependency(msg) => write!(f, "Circular dependency: {}", msg),
            RelationshipError::ForeignKeyViolation(msg) => write!(f, "Foreign key violation: {}", msg),
        }
    }
}

impl std::error::Error for RelationshipError {}

impl From<RelationshipError> for ModelError {
    fn from(err: RelationshipError) -> Self {
        ModelError::Relationship(err.to_string())
    }
}

/// Error types for migration operations
#[derive(Debug, Clone)]
pub enum MigrationError {
    /// Migration file not found
    FileNotFound(String),
    /// Invalid migration syntax
    InvalidSyntax(String),
    /// Migration already applied
    AlreadyApplied(String),
    /// Migration rollback failed
    RollbackFailed(String),
    /// Version conflict
    VersionConflict(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::FileNotFound(msg) => write!(f, "Migration file not found: {}", msg),
            MigrationError::InvalidSyntax(msg) => write!(f, "Invalid migration syntax: {}", msg),
            MigrationError::AlreadyApplied(msg) => write!(f, "Migration already applied: {}", msg),
            MigrationError::RollbackFailed(msg) => write!(f, "Migration rollback failed: {}", msg),
            MigrationError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<MigrationError> for ModelError {
    fn from(err: MigrationError) -> Self {
        ModelError::Migration(err.to_string())
    }
}

/// Error types for event system operations
#[derive(Debug, Clone)]
pub enum EventError {
    /// Event handler failed
    HandlerFailed(String),
    /// Event propagation stopped
    PropagationStopped(String),
    /// Invalid event configuration
    InvalidConfiguration(String),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventError::HandlerFailed(msg) => write!(f, "Event handler failed: {}", msg),
            EventError::PropagationStopped(msg) => write!(f, "Event propagation stopped: {}", msg),
            EventError::InvalidConfiguration(msg) => write!(f, "Invalid event configuration: {}", msg),
        }
    }
}

impl std::error::Error for EventError {}

impl From<EventError> for ModelError {
    fn from(err: EventError) -> Self {
        ModelError::Event(err.to_string())
    }
}
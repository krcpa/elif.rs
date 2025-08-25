//! Primary Key System - Types and implementations for model primary keys
//!
//! Supports integer, UUID, and composite primary keys with proper serialization,
//! display formatting, and type conversion utilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Primary key types supported by the ORM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimaryKey {
    /// Auto-incrementing integer primary key
    Integer(i64),
    /// UUID primary key
    Uuid(Uuid),
    /// Composite primary key (multiple fields)
    Composite(HashMap<String, String>),
}

impl std::fmt::Display for PrimaryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryKey::Integer(id) => write!(f, "{}", id),
            PrimaryKey::Uuid(id) => write!(f, "{}", id),
            PrimaryKey::Composite(fields) => {
                let pairs: Vec<String> =
                    fields.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
                write!(f, "{}", pairs.join(","))
            }
        }
    }
}

impl Default for PrimaryKey {
    fn default() -> Self {
        PrimaryKey::Integer(0)
    }
}

impl PrimaryKey {
    /// Extract as i64 if this is an Integer primary key
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PrimaryKey::Integer(id) => Some(*id),
            _ => None,
        }
    }

    /// Extract as UUID if this is a UUID primary key
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PrimaryKey::Uuid(id) => Some(*id),
            _ => None,
        }
    }

    /// Extract as composite fields if this is a Composite primary key
    pub fn as_composite(&self) -> Option<&HashMap<String, String>> {
        match self {
            PrimaryKey::Composite(fields) => Some(fields),
            _ => None,
        }
    }

    /// Check if this is a valid (non-default) primary key
    pub fn is_valid(&self) -> bool {
        match self {
            PrimaryKey::Integer(0) => false,
            PrimaryKey::Integer(_) => true,
            PrimaryKey::Uuid(uuid) => !uuid.is_nil(),
            PrimaryKey::Composite(fields) => !fields.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_key_display() {
        let int_key = PrimaryKey::Integer(123);
        assert_eq!(format!("{}", int_key), "123");

        let uuid_key =
            PrimaryKey::Uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap());
        assert_eq!(
            format!("{}", uuid_key),
            "550e8400-e29b-41d4-a716-446655440000"
        );

        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), "1".to_string());
        fields.insert("role_id".to_string(), "2".to_string());
        let composite_key = PrimaryKey::Composite(fields);
        let display_str = format!("{}", composite_key);
        assert!(display_str.contains("user_id:1") && display_str.contains("role_id:2"));
    }

    #[test]
    fn test_primary_key_validation() {
        assert!(!PrimaryKey::Integer(0).is_valid());
        assert!(PrimaryKey::Integer(1).is_valid());

        assert!(!PrimaryKey::Uuid(Uuid::nil()).is_valid());
        assert!(PrimaryKey::Uuid(Uuid::new_v4()).is_valid());

        assert!(!PrimaryKey::Composite(HashMap::new()).is_valid());
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), "1".to_string());
        assert!(PrimaryKey::Composite(fields).is_valid());
    }
}

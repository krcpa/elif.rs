//! Mapping and Hydration Tests
//!
//! Tests model mapping from database rows without requiring a real database connection.
//! Uses mock database rows to test the hydration and serialization functionality.

use crate::{Model, ModelResult, ModelError, PrimaryKey, OrmResult};
use crate::backends::{DatabaseRow, DatabaseValue, DatabaseRowExt};
use serde_json::Value as JsonValue;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

/// Mock database row implementation for testing
pub struct MockDatabaseRow {
    columns: HashMap<String, Value>,
}

impl MockDatabaseRow {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }
    
    pub fn with_column<T: Into<Value>>(mut self, name: &str, value: T) -> Self {
        self.columns.insert(name.to_string(), value.into());
        self
    }
    
    pub fn get_column<T>(&self, name: &str) -> Result<T, ModelError>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.columns.get(name) {
            Some(value) => {
                serde_json::from_value(value.clone())
                    .map_err(|e| ModelError::Serialization(format!("Failed to deserialize column '{}': {}", name, e)))
            },
            None => Err(ModelError::ColumnNotFound(format!("Column '{}' not found", name)))
        }
    }
}

impl DatabaseRow for MockDatabaseRow {
    fn get_by_index(&self, index: usize) -> OrmResult<DatabaseValue> {
        let keys: Vec<_> = self.columns.keys().collect();
        if let Some(key) = keys.get(index) {
            let value = self.columns.get(*key).unwrap();
            Ok(DatabaseValue::from_json(value.clone()))
        } else {
            Err(crate::ModelError::ColumnNotFound(format!("Column at index {} not found", index)))
        }
    }

    fn get_by_name(&self, name: &str) -> OrmResult<DatabaseValue> {
        match self.columns.get(name) {
            Some(value) => Ok(DatabaseValue::from_json(value.clone())),
            None => Err(crate::ModelError::ColumnNotFound(format!("Column '{}' not found", name)))
        }
    }

    fn column_count(&self) -> usize {
        self.columns.len()
    }

    fn column_names(&self) -> Vec<String> {
        self.columns.keys().cloned().collect()
    }

    fn to_json(&self) -> OrmResult<JsonValue> {
        Ok(serde_json::to_value(&self.columns).unwrap())
    }

    fn to_map(&self) -> OrmResult<HashMap<String, DatabaseValue>> {
        let mut map = HashMap::new();
        for (key, value) in &self.columns {
            map.insert(key.clone(), DatabaseValue::from_json(value.clone()));
        }
        Ok(map)
    }
}

// Test models for mapping tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Model for TestUser {
    type PrimaryKey = Uuid;
    
    fn table_name() -> &'static str {
        "users"
    }
    
    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }
    
    fn to_fields(&self) -> ModelResult<HashMap<String, Value>> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(self.id));
        fields.insert("name".to_string(), json!(self.name));
        fields.insert("email".to_string(), json!(self.email));
        fields.insert("age".to_string(), json!(self.age));
        fields.insert("active".to_string(), json!(self.active));
        fields.insert("created_at".to_string(), json!(self.created_at));
        fields.insert("updated_at".to_string(), json!(self.updated_at));
        Ok(fields)
    }
    
    fn from_row(row: &dyn DatabaseRow) -> ModelResult<Self> {
        Ok(TestUser {
            id: row.get("id")?,
            name: row.get("name")?,
            email: row.get("email")?,
            age: row.try_get("age")?.flatten(),
            active: row.get("active")?,
            created_at: row.get("created_at")?,
            updated_at: row.try_get("updated_at")?.flatten(),
        })
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = key;
    }
}

impl PrimaryKey<Uuid> for TestUser {
    fn primary_key_type() -> crate::PrimaryKeyType {
        crate::PrimaryKeyType::Uuid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestPost {
    pub id: i32,
    pub user_id: Uuid,
    pub title: String,
    pub content: Option<String>,
    pub published: bool,
    pub view_count: i64,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Model for TestPost {
    type PrimaryKey = i32;
    
    fn table_name() -> &'static str {
        "posts"
    }
    
    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }
    
    fn to_fields(&self) -> ModelResult<HashMap<String, Value>> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), json!(self.id));
        fields.insert("user_id".to_string(), json!(self.user_id));
        fields.insert("title".to_string(), json!(self.title));
        fields.insert("content".to_string(), json!(self.content));
        fields.insert("published".to_string(), json!(self.published));
        fields.insert("view_count".to_string(), json!(self.view_count));
        fields.insert("tags".to_string(), json!(self.tags));
        fields.insert("metadata".to_string(), self.metadata.clone());
        fields.insert("created_at".to_string(), json!(self.created_at));
        Ok(fields)
    }
    
    fn from_row(row: &dyn DatabaseRow) -> ModelResult<Self> {
        Ok(TestPost {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            title: row.get("title")?,
            content: row.try_get("content")?.flatten(),
            published: row.get("published")?,
            view_count: row.get("view_count")?,
            tags: row.get("tags")?,
            metadata: row.get("metadata")?,
            created_at: row.get("created_at")?,
        })
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = key;
    }
}

impl PrimaryKey<i32> for TestPost {
    fn primary_key_type() -> crate::PrimaryKeyType {
        crate::PrimaryKeyType::Integer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_mock_database_row_creation() {
        let row = MockDatabaseRow::new()
            .with_column("id", 1)
            .with_column("name", "Test User")
            .with_column("active", true);
        
        assert_eq!(row.get::<i32>("id").unwrap(), 1);
        assert_eq!(row.get::<String>("name").unwrap(), "Test User");
        assert_eq!(row.get::<bool>("active").unwrap(), true);
    }
    
    #[test]
    fn test_mock_database_row_missing_column() {
        let row = MockDatabaseRow::new()
            .with_column("id", 1);
        
        let result = row.get::<String>("missing_column");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ModelError::ColumnNotFound(_)));
    }
    
    #[test]
    fn test_mock_database_row_null_handling() {
        let row = MockDatabaseRow::new()
            .with_column("id", 1)
            .with_column("nullable_field", Value::Null);
        
        assert_eq!(row.get::<i32>("id").unwrap(), 1);
        assert_eq!(row.try_get::<String>("nullable_field").unwrap(), None);
    }
    
    #[test]
    fn test_user_model_from_row() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = MockDatabaseRow::new()
            .with_column("id", user_id.to_string())
            .with_column("name", "John Doe")
            .with_column("email", "john@example.com")
            .with_column("age", 30)
            .with_column("active", true)
            .with_column("created_at", now.to_rfc3339())
            .with_column("updated_at", Value::Null);
        
        let user = TestUser::from_row(&row).unwrap();
        
        assert_eq!(user.id, user_id);
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        assert_eq!(user.age, Some(30));
        assert_eq!(user.active, true);
        assert_eq!(user.updated_at, None);
    }
    
    #[test]
    fn test_user_model_to_fields() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let user = TestUser {
            id: user_id,
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            age: Some(25),
            active: true,
            created_at: now,
            updated_at: Some(now),
        };
        
        let fields = user.to_fields().unwrap();
        
        assert_eq!(fields.get("id").unwrap(), &json!(user_id));
        assert_eq!(fields.get("name").unwrap(), "Jane Smith");
        assert_eq!(fields.get("email").unwrap(), "jane@example.com");
        assert_eq!(fields.get("age").unwrap(), &json!(25));
        assert_eq!(fields.get("active").unwrap(), &json!(true));
    }
    
    #[test]
    fn test_post_model_with_complex_types() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = MockDatabaseRow::new()
            .with_column("id", 1)
            .with_column("user_id", user_id.to_string())
            .with_column("title", "Test Post")
            .with_column("content", "This is test content")
            .with_column("published", true)
            .with_column("view_count", 150)
            .with_column("tags", json!(["rust", "orm", "testing"]))
            .with_column("metadata", json!({"priority": "high", "featured": true}))
            .with_column("created_at", now.to_rfc3339());
        
        let post = TestPost::from_row(&row).unwrap();
        
        assert_eq!(post.id, 1);
        assert_eq!(post.user_id, user_id);
        assert_eq!(post.title, "Test Post");
        assert_eq!(post.content, Some("This is test content".to_string()));
        assert_eq!(post.published, true);
        assert_eq!(post.view_count, 150);
        assert_eq!(post.tags, vec!["rust", "orm", "testing"]);
        assert_eq!(post.metadata["priority"], "high");
        assert_eq!(post.metadata["featured"], true);
    }
    
    #[test]
    fn test_post_model_with_optional_content() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = MockDatabaseRow::new()
            .with_column("id", 2)
            .with_column("user_id", user_id.to_string())
            .with_column("title", "Draft Post")
            .with_column("content", Value::Null) // Test null content
            .with_column("published", false)
            .with_column("view_count", 0)
            .with_column("tags", json!([]))
            .with_column("metadata", json!({}))
            .with_column("created_at", now.to_rfc3339());
        
        let post = TestPost::from_row(&row).unwrap();
        
        assert_eq!(post.id, 2);
        assert_eq!(post.title, "Draft Post");
        assert_eq!(post.content, None);
        assert_eq!(post.published, false);
        assert_eq!(post.tags, Vec::<String>::new());
    }
    
    #[test]
    fn test_model_serialization_roundtrip() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let original_user = TestUser {
            id: user_id,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            age: Some(35),
            active: true,
            created_at: now,
            updated_at: Some(now),
        };
        
        // Convert to fields (as if saving to database)
        let fields = original_user.to_fields().unwrap();
        
        // Create mock row from fields (as if loading from database)
        let mut mock_row = MockDatabaseRow::new();
        for (key, value) in fields {
            mock_row = mock_row.with_column(&key, value);
        }
        
        // Convert back to model
        let restored_user = TestUser::from_row(&mock_row).unwrap();
        
        assert_eq!(original_user, restored_user);
    }
    
    #[test]
    fn test_model_primary_key_handling() {
        let user_id = Uuid::new_v4();
        let user = TestUser {
            id: user_id,
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            age: None,
            active: true,
            created_at: Utc::now(),
            updated_at: None,
        };
        
        assert_eq!(user.primary_key(), Some(user_id));
        assert_eq!(TestUser::table_name(), "users");
        assert_eq!(TestUser::primary_key_type(), crate::PrimaryKeyType::Uuid);
        
        let post = TestPost {
            id: 42,
            user_id: user_id,
            title: "Test Post".to_string(),
            content: None,
            published: false,
            view_count: 0,
            tags: vec![],
            metadata: json!({}),
            created_at: Utc::now(),
        };
        
        assert_eq!(post.primary_key(), Some(42));
        assert_eq!(TestPost::table_name(), "posts");
        assert_eq!(TestPost::primary_key_type(), crate::PrimaryKeyType::Integer);
    }
    
    #[test]
    fn test_model_type_safety() {
        let row = MockDatabaseRow::new()
            .with_column("id", "not-a-uuid")
            .with_column("name", "Test User")
            .with_column("email", "test@example.com")
            .with_column("age", Value::Null)
            .with_column("active", true)
            .with_column("created_at", Utc::now().to_rfc3339());
        
        // Should fail due to invalid UUID
        let result = TestUser::from_row(&row);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ModelError::Serialization(_)));
    }
    
    #[test]
    fn test_complex_json_mapping() {
        let complex_metadata = json!({
            "author": {
                "name": "John Doe",
                "id": 123
            },
            "tags": ["rust", "programming"],
            "settings": {
                "public": true,
                "comments_enabled": false
            },
            "stats": {
                "likes": 25,
                "shares": 5
            }
        });
        
        let row = MockDatabaseRow::new()
            .with_column("id", 1)
            .with_column("user_id", Uuid::new_v4().to_string())
            .with_column("title", "Complex Post")
            .with_column("content", Value::Null)
            .with_column("published", true)
            .with_column("view_count", 100)
            .with_column("tags", json!(["complex", "json"]))
            .with_column("metadata", complex_metadata.clone())
            .with_column("created_at", Utc::now().to_rfc3339());
        
        let post = TestPost::from_row(&row).unwrap();
        
        assert_eq!(post.metadata, complex_metadata);
        assert_eq!(post.metadata["author"]["name"], "John Doe");
        assert_eq!(post.metadata["stats"]["likes"], 25);
    }
}
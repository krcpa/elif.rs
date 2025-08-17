use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource specification for code generation and API definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub kind: String,
    pub name: String,
    pub route: String,
    pub storage: StorageSpec,
    #[serde(default)]
    pub indexes: Vec<IndexSpec>,
    #[serde(default)]
    pub uniques: Vec<UniqueSpec>,
    #[serde(default)]
    pub relations: Vec<RelationSpec>,
    pub api: ApiSpec,
    #[serde(default)]
    pub policy: PolicySpec,
    #[serde(default)]
    pub validate: ValidateSpec,
    #[serde(default)]
    pub examples: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub events: EventSpec,
}

impl ResourceSpec {
    /// Create a resource spec from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
    
    /// Convert resource spec to YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
    
    /// Create a resource spec from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    /// Convert resource spec to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Get the table name for this resource
    pub fn table_name(&self) -> &str {
        &self.storage.table
    }
    
    /// Get the primary key field specification
    pub fn primary_key(&self) -> Option<&FieldSpec> {
        self.storage.fields.iter().find(|f| f.pk)
    }
    
    /// Get all required fields
    pub fn required_fields(&self) -> Vec<&FieldSpec> {
        self.storage.fields.iter().filter(|f| f.required).collect()
    }
    
    /// Get all indexed fields
    pub fn indexed_fields(&self) -> Vec<&FieldSpec> {
        self.storage.fields.iter().filter(|f| f.index).collect()
    }
    
    /// Check if resource has soft delete enabled
    pub fn has_soft_delete(&self) -> bool {
        self.storage.soft_delete
    }
    
    /// Check if resource has timestamps enabled
    pub fn has_timestamps(&self) -> bool {
        self.storage.timestamps
    }
}

/// Storage specification for database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSpec {
    pub table: String,
    #[serde(default)]
    pub soft_delete: bool,
    #[serde(default = "default_true")]
    pub timestamps: bool,
    pub fields: Vec<FieldSpec>,
}

/// Field specification for database columns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub pk: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub index: bool,
    pub default: Option<String>,
    pub validate: Option<ValidationRule>,
}

impl FieldSpec {
    /// Check if field is primary key
    pub fn is_primary_key(&self) -> bool {
        self.pk
    }
    
    /// Check if field is required
    pub fn is_required(&self) -> bool {
        self.required
    }
    
    /// Check if field is indexed
    pub fn is_indexed(&self) -> bool {
        self.index
    }
    
    /// Check if field has a default value
    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }
}

/// Validation rule for fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub pattern: Option<String>,
}

/// Index specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSpec {
    pub name: String,
    pub fields: Vec<String>,
}

/// Unique constraint specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueSpec {
    pub name: String,
    pub fields: Vec<String>,
}

/// Relation specification for foreign keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSpec {
    pub name: String,
    pub target: String,
    pub relation_type: String,
}

impl RelationSpec {
    /// Check if relation is one-to-one
    pub fn is_one_to_one(&self) -> bool {
        self.relation_type == "one_to_one" || self.relation_type == "1:1"
    }
    
    /// Check if relation is one-to-many
    pub fn is_one_to_many(&self) -> bool {
        self.relation_type == "one_to_many" || self.relation_type == "1:many"
    }
    
    /// Check if relation is many-to-many
    pub fn is_many_to_many(&self) -> bool {
        self.relation_type == "many_to_many" || self.relation_type == "many:many"
    }
}

/// API specification for endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSpec {
    pub operations: Vec<OperationSpec>,
}

/// Operation specification for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSpec {
    pub op: String,
    pub method: String,
    pub path: String,
    pub paging: Option<String>,
    pub filter: Option<Vec<String>>,
    pub search_by: Option<Vec<String>>,
    pub order_by: Option<Vec<String>>,
}

impl OperationSpec {
    /// Check if operation supports paging
    pub fn supports_paging(&self) -> bool {
        self.paging.is_some()
    }
    
    /// Check if operation supports filtering
    pub fn supports_filtering(&self) -> bool {
        self.filter.as_ref().map_or(false, |f| !f.is_empty())
    }
    
    /// Check if operation supports searching
    pub fn supports_searching(&self) -> bool {
        self.search_by.as_ref().map_or(false, |s| !s.is_empty())
    }
    
    /// Check if operation supports ordering
    pub fn supports_ordering(&self) -> bool {
        self.order_by.as_ref().map_or(false, |o| !o.is_empty())
    }
}

/// Policy specification for access control
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySpec {
    #[serde(default = "default_public")]
    pub auth: String,
    pub rate_limit: Option<String>,
}

/// Validation specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateSpec {
    #[serde(default)]
    pub constraints: Vec<ConstraintSpec>,
}

/// Constraint specification for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSpec {
    pub rule: String,
    pub code: String,
    pub hint: String,
}

/// Event specification for event handling
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventSpec {
    #[serde(default)]
    pub emit: Vec<String>,
}

// Helper functions for serde defaults
fn default_true() -> bool {
    true
}

fn default_public() -> String {
    "public".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_spec_yaml() {
        let yaml = r#"
kind: Resource
name: User
route: /users
storage:
  table: users
  fields:
    - name: id
      type: uuid
      pk: true
    - name: name
      type: string
      required: true
api:
  operations:
    - op: list
      method: GET
      path: /
"#;
        
        let spec = ResourceSpec::from_yaml(yaml).unwrap();
        assert_eq!(spec.name, "User");
        assert_eq!(spec.storage.table, "users");
        assert_eq!(spec.storage.fields.len(), 2);
        
        let yaml_output = spec.to_yaml().unwrap();
        assert!(yaml_output.contains("name: User"));
    }
    
    #[test]
    fn test_field_spec_helpers() {
        let field = FieldSpec {
            name: "id".to_string(),
            field_type: "uuid".to_string(),
            pk: true,
            required: true,
            index: true,
            default: Some("gen_random_uuid()".to_string()),
            validate: None,
        };
        
        assert!(field.is_primary_key());
        assert!(field.is_required());
        assert!(field.is_indexed());
        assert!(field.has_default());
    }
    
    #[test]
    fn test_relation_spec_types() {
        let relation = RelationSpec {
            name: "posts".to_string(),
            target: "Post".to_string(),
            relation_type: "one_to_many".to_string(),
        };
        
        assert!(relation.is_one_to_many());
        assert!(!relation.is_one_to_one());
        assert!(!relation.is_many_to_many());
    }
}
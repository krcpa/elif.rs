use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSpec {
    pub table: String,
    #[serde(default)]
    pub soft_delete: bool,
    #[serde(default = "default_true")]
    pub timestamps: bool,
    pub fields: Vec<FieldSpec>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSpec {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueSpec {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSpec {
    pub name: String,
    pub target: String,
    pub relation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSpec {
    pub operations: Vec<OperationSpec>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySpec {
    #[serde(default = "default_public")]
    pub auth: String,
    pub rate_limit: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateSpec {
    #[serde(default)]
    pub constraints: Vec<ConstraintSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSpec {
    pub rule: String,
    pub code: String,
    pub hint: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventSpec {
    #[serde(default)]
    pub emit: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_public() -> String {
    "public".to_string()
}

impl ResourceSpec {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
    
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
    
    pub fn table_name(&self) -> &str {
        &self.storage.table
    }
    
    pub fn primary_key(&self) -> Option<&FieldSpec> {
        self.storage.fields.iter().find(|f| f.pk)
    }
}
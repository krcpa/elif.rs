//! Relationship Metadata System - Core metadata definitions for relationships

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::{ModelError, ModelResult};

/// Defines the type of relationship between models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    /// One-to-one relationship (hasOne)
    HasOne,
    /// One-to-many relationship (hasMany)
    HasMany,
    /// Many-to-one relationship (belongsTo)
    BelongsTo,
    /// Many-to-many relationship through a pivot table
    ManyToMany,
    /// Polymorphic one-to-one relationship
    MorphOne,
    /// Polymorphic one-to-many relationship
    MorphMany,
    /// Inverse polymorphic relationship
    MorphTo,
}

impl RelationshipType {
    /// Returns true if this relationship type is polymorphic
    pub fn is_polymorphic(self) -> bool {
        matches!(self, Self::MorphOne | Self::MorphMany | Self::MorphTo)
    }

    /// Returns true if this relationship returns a collection
    pub fn is_collection(self) -> bool {
        matches!(self, Self::HasMany | Self::ManyToMany | Self::MorphMany)
    }

    /// Returns true if this relationship requires a pivot table
    pub fn requires_pivot(self) -> bool {
        matches!(self, Self::ManyToMany)
    }
}

/// Comprehensive relationship metadata containing all necessary information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipMetadata {
    /// The type of relationship
    pub relationship_type: RelationshipType,
    
    /// Name of the relationship (field name in the model)
    pub name: String,
    
    /// The related model's table name
    pub related_table: String,
    
    /// The related model's type name
    pub related_model: String,
    
    /// Foreign key configuration
    pub foreign_key: ForeignKeyConfig,
    
    /// Local key (primary key on this model, defaults to "id")
    pub local_key: String,
    
    /// Optional custom relationship name for queries
    pub custom_name: Option<String>,
    
    /// Pivot table configuration for many-to-many relationships
    pub pivot_config: Option<PivotConfig>,
    
    /// Polymorphic configuration
    pub polymorphic_config: Option<PolymorphicConfig>,
    
    /// Whether this relationship should be eagerly loaded by default
    pub eager_load: bool,
    
    /// Additional constraints for the relationship
    pub constraints: Vec<RelationshipConstraint>,
    
    /// Inverse relationship name (for automatic detection)
    pub inverse: Option<String>,
}

impl RelationshipMetadata {
    /// Create a new RelationshipMetadata instance
    pub fn new(
        relationship_type: RelationshipType,
        name: String,
        related_table: String,
        related_model: String,
        foreign_key: ForeignKeyConfig,
    ) -> Self {
        Self {
            relationship_type,
            name,
            related_table,
            related_model,
            foreign_key,
            local_key: "id".to_string(),
            custom_name: None,
            pivot_config: None,
            polymorphic_config: None,
            eager_load: false,
            constraints: Vec::new(),
            inverse: None,
        }
    }

    /// Set the local key (primary key on this model)
    pub fn with_local_key(mut self, local_key: String) -> Self {
        self.local_key = local_key;
        self
    }

    /// Set a custom name for the relationship
    pub fn with_custom_name(mut self, custom_name: String) -> Self {
        self.custom_name = Some(custom_name);
        self
    }

    /// Set pivot table configuration
    pub fn with_pivot(mut self, pivot_config: PivotConfig) -> Self {
        self.pivot_config = Some(pivot_config);
        self
    }

    /// Set polymorphic configuration
    pub fn with_polymorphic(mut self, polymorphic_config: PolymorphicConfig) -> Self {
        self.polymorphic_config = Some(polymorphic_config);
        self
    }

    /// Enable eager loading by default
    pub fn with_eager_load(mut self, eager_load: bool) -> Self {
        self.eager_load = eager_load;
        self
    }

    /// Add constraints to the relationship
    pub fn with_constraints(mut self, constraints: Vec<RelationshipConstraint>) -> Self {
        self.constraints = constraints;
        self
    }

    /// Set the inverse relationship name
    pub fn with_inverse(mut self, inverse: String) -> Self {
        self.inverse = Some(inverse);
        self
    }

    /// Validate the relationship metadata for consistency
    pub fn validate(&self) -> ModelResult<()> {
        // Check if relationship type matches configuration
        if self.relationship_type.requires_pivot() && self.pivot_config.is_none() {
            return Err(ModelError::Configuration(
                format!("Relationship '{}' of type {:?} requires pivot configuration", 
                        self.name, self.relationship_type)
            ));
        }

        if self.relationship_type.is_polymorphic() && self.polymorphic_config.is_none() {
            return Err(ModelError::Configuration(
                format!("Relationship '{}' of type {:?} requires polymorphic configuration", 
                        self.name, self.relationship_type)
            ));
        }

        // Validate foreign key configuration
        self.foreign_key.validate()?;

        // Validate pivot configuration if present
        if let Some(ref pivot) = self.pivot_config {
            pivot.validate()?;
        }

        // Validate polymorphic configuration if present
        if let Some(ref poly) = self.polymorphic_config {
            poly.validate()?;
        }

        Ok(())
    }

    /// Get the effective relationship name for queries
    pub fn query_name(&self) -> &str {
        self.custom_name.as_ref().unwrap_or(&self.name)
    }
}

/// Foreign key configuration for relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyConfig {
    /// The foreign key column name(s)
    pub columns: Vec<String>,
    
    /// Whether this is a composite foreign key
    pub is_composite: bool,
    
    /// The table where the foreign key is located
    pub table: String,
}

impl ForeignKeyConfig {
    /// Create a simple foreign key configuration
    pub fn simple(column: String, table: String) -> Self {
        Self {
            columns: vec![column],
            is_composite: false,
            table,
        }
    }

    /// Create a composite foreign key configuration
    pub fn composite(columns: Vec<String>, table: String) -> Self {
        Self {
            columns,
            is_composite: true,
            table,
        }
    }

    /// Get the primary foreign key column (first in composite keys)
    pub fn primary_column(&self) -> &str {
        self.columns.first().map(|s| s.as_str()).unwrap_or("")
    }

    /// Validate the foreign key configuration
    pub fn validate(&self) -> ModelResult<()> {
        if self.columns.is_empty() {
            return Err(ModelError::Configuration(
                "Foreign key configuration must have at least one column".to_string()
            ));
        }

        if self.is_composite && self.columns.len() < 2 {
            return Err(ModelError::Configuration(
                "Composite foreign key must have at least 2 columns".to_string()
            ));
        }

        if self.table.is_empty() {
            return Err(ModelError::Configuration(
                "Foreign key configuration must specify a table".to_string()
            ));
        }

        Ok(())
    }
}

/// Pivot table configuration for many-to-many relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotConfig {
    /// The pivot table name
    pub table: String,
    
    /// The foreign key column for the local model in the pivot table
    pub local_key: String,
    
    /// The foreign key column for the related model in the pivot table
    pub foreign_key: String,
    
    /// Additional columns to include from the pivot table
    pub additional_columns: Vec<String>,
    
    /// Timestamps configuration for the pivot table
    pub with_timestamps: bool,
}

impl PivotConfig {
    /// Create a new pivot configuration
    pub fn new(table: String, local_key: String, foreign_key: String) -> Self {
        Self {
            table,
            local_key,
            foreign_key,
            additional_columns: Vec::new(),
            with_timestamps: false,
        }
    }

    /// Add additional columns to select from the pivot table
    pub fn with_additional_columns(mut self, columns: Vec<String>) -> Self {
        self.additional_columns = columns;
        self
    }

    /// Enable timestamp columns on the pivot table
    pub fn with_timestamps(mut self) -> Self {
        self.with_timestamps = true;
        self
    }

    /// Validate the pivot configuration
    pub fn validate(&self) -> ModelResult<()> {
        if self.table.is_empty() {
            return Err(ModelError::Configuration(
                "Pivot table name cannot be empty".to_string()
            ));
        }

        if self.local_key.is_empty() {
            return Err(ModelError::Configuration(
                "Pivot local key cannot be empty".to_string()
            ));
        }

        if self.foreign_key.is_empty() {
            return Err(ModelError::Configuration(
                "Pivot foreign key cannot be empty".to_string()
            ));
        }

        if self.local_key == self.foreign_key {
            return Err(ModelError::Configuration(
                "Pivot local key and foreign key must be different".to_string()
            ));
        }

        Ok(())
    }
}

/// Polymorphic relationship configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolymorphicConfig {
    /// The morph type column name (stores the model type)
    pub type_column: String,
    
    /// The morph id column name (stores the foreign key)
    pub id_column: String,
    
    /// The name/namespace for this polymorphic relationship
    pub name: String,
    
    /// Allowed types for this polymorphic relationship
    pub allowed_types: Vec<String>,
}

impl PolymorphicConfig {
    /// Create a new polymorphic configuration
    pub fn new(name: String, type_column: String, id_column: String) -> Self {
        Self {
            type_column,
            id_column,
            name,
            allowed_types: Vec::new(),
        }
    }

    /// Set allowed types for the polymorphic relationship
    pub fn with_allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = types;
        self
    }

    /// Validate the polymorphic configuration
    pub fn validate(&self) -> ModelResult<()> {
        if self.name.is_empty() {
            return Err(ModelError::Configuration(
                "Polymorphic relationship name cannot be empty".to_string()
            ));
        }

        if self.type_column.is_empty() {
            return Err(ModelError::Configuration(
                "Polymorphic type column cannot be empty".to_string()
            ));
        }

        if self.id_column.is_empty() {
            return Err(ModelError::Configuration(
                "Polymorphic ID column cannot be empty".to_string()
            ));
        }

        if self.type_column == self.id_column {
            return Err(ModelError::Configuration(
                "Polymorphic type column and ID column must be different".to_string()
            ));
        }

        Ok(())
    }
}

/// Relationship constraint for additional filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipConstraint {
    /// The column to constrain
    pub column: String,
    
    /// The constraint operator
    pub operator: ConstraintOperator,
    
    /// The constraint value
    pub value: String,
}

/// Constraint operators for relationship constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    In,
    NotIn,
    Like,
    NotLike,
    IsNull,
    IsNotNull,
}

impl ConstraintOperator {
    /// Convert the operator to its SQL representation
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThanOrEqual => "<=",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::Like => "LIKE",
            Self::NotLike => "NOT LIKE",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_type_properties() {
        assert!(RelationshipType::MorphOne.is_polymorphic());
        assert!(RelationshipType::MorphMany.is_polymorphic());
        assert!(RelationshipType::MorphTo.is_polymorphic());
        assert!(!RelationshipType::HasOne.is_polymorphic());

        assert!(RelationshipType::HasMany.is_collection());
        assert!(RelationshipType::ManyToMany.is_collection());
        assert!(RelationshipType::MorphMany.is_collection());
        assert!(!RelationshipType::HasOne.is_collection());

        assert!(RelationshipType::ManyToMany.requires_pivot());
        assert!(!RelationshipType::HasMany.requires_pivot());
    }

    #[test]
    fn test_relationship_metadata_creation() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );

        assert_eq!(metadata.relationship_type, RelationshipType::HasMany);
        assert_eq!(metadata.name, "posts");
        assert_eq!(metadata.related_table, "posts");
        assert_eq!(metadata.local_key, "id");
        assert!(!metadata.eager_load);
    }

    #[test]
    fn test_relationship_metadata_validation() {
        // Valid has-many relationship
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );
        assert!(metadata.validate().is_ok());

        // Invalid many-to-many without pivot config
        let invalid_metadata = RelationshipMetadata::new(
            RelationshipType::ManyToMany,
            "roles".to_string(),
            "roles".to_string(),
            "Role".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "user_roles".to_string()),
        );
        assert!(invalid_metadata.validate().is_err());
    }

    #[test]
    fn test_foreign_key_config() {
        let simple_fk = ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string());
        assert!(!simple_fk.is_composite);
        assert_eq!(simple_fk.primary_column(), "user_id");
        assert!(simple_fk.validate().is_ok());

        let composite_fk = ForeignKeyConfig::composite(
            vec!["user_id".to_string(), "company_id".to_string()],
            "posts".to_string(),
        );
        assert!(composite_fk.is_composite);
        assert_eq!(composite_fk.primary_column(), "user_id");
        assert!(composite_fk.validate().is_ok());
    }

    #[test]
    fn test_pivot_config() {
        let pivot = PivotConfig::new(
            "user_roles".to_string(),
            "user_id".to_string(),
            "role_id".to_string(),
        ).with_timestamps();

        assert_eq!(pivot.table, "user_roles");
        assert_eq!(pivot.local_key, "user_id");
        assert_eq!(pivot.foreign_key, "role_id");
        assert!(pivot.with_timestamps);
        assert!(pivot.validate().is_ok());
    }

    #[test]
    fn test_polymorphic_config() {
        let poly = PolymorphicConfig::new(
            "commentable".to_string(),
            "commentable_type".to_string(),
            "commentable_id".to_string(),
        ).with_allowed_types(vec!["Post".to_string(), "Video".to_string()]);

        assert_eq!(poly.name, "commentable");
        assert_eq!(poly.type_column, "commentable_type");
        assert_eq!(poly.id_column, "commentable_id");
        assert_eq!(poly.allowed_types.len(), 2);
        assert!(poly.validate().is_ok());
    }

    #[test]
    fn test_constraint_operator_sql() {
        assert_eq!(ConstraintOperator::Equal.to_sql(), "=");
        assert_eq!(ConstraintOperator::In.to_sql(), "IN");
        assert_eq!(ConstraintOperator::IsNull.to_sql(), "IS NULL");
    }

    #[test]
    fn test_relationship_metadata_builder_pattern() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        )
        .with_local_key("uuid".to_string())
        .with_custom_name("user_profile".to_string())
        .with_eager_load(true)
        .with_inverse("user".to_string());

        assert_eq!(metadata.local_key, "uuid");
        assert_eq!(metadata.custom_name, Some("user_profile".to_string()));
        assert!(metadata.eager_load);
        assert_eq!(metadata.inverse, Some("user".to_string()));
        assert_eq!(metadata.query_name(), "user_profile");
    }
}
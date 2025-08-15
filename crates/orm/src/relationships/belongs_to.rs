//! BelongsTo Relationship - Clean implementation using new query system

use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;

use crate::error::ModelResult;
use crate::model::Model;
use crate::query::QueryBuilder;

use super::traits::{Relationship, RelationshipMeta};

/// BelongsTo relationship - child model belongs to a parent model
pub struct BelongsTo<Child, Parent>
where
    Child: Model + Send + Sync,
    Parent: Model + Send + Sync,
{
    child: Child,
    parent: Option<Parent>,
    meta: RelationshipMeta,
    loaded: bool,
}

impl<Child, Parent> BelongsTo<Child, Parent>
where
    Child: Model + Send + Sync,
    Parent: Model + Send + Sync,
{
    /// Create a new BelongsTo relationship
    pub fn new(child: Child, foreign_key: &str) -> Self {
        Self {
            child,
            parent: None,
            meta: RelationshipMeta {
                foreign_key: foreign_key.to_string(),
                local_key: Parent::primary_key_name().to_string(),
                related_table: Parent::table_name().to_string(),
            },
            loaded: false,
        }
    }

    /// Get the parent model if loaded
    pub fn get(&self) -> Option<&Parent> {
        self.parent.as_ref()
    }

    /// Get the parent model as mutable if loaded
    pub fn get_mut(&mut self) -> Option<&mut Parent> {
        self.parent.as_mut()
    }

    /// Take ownership of the parent model
    pub fn take(&mut self) -> Option<Parent> {
        self.parent.take()
    }

    /// Set the parent model
    pub fn set(&mut self, parent: Option<Parent>) {
        self.parent = parent;
        self.loaded = true;
    }

    /// Get the foreign key value from the child model
    pub fn foreign_key_value(&self) -> Option<String> {
        self.child.to_fields().get(&self.meta.foreign_key)
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
    }

    /// Check if the relationship has a foreign key value
    pub fn has_foreign_key(&self) -> bool {
        self.foreign_key_value().is_some()
    }
}

#[async_trait]
impl<Child, Parent> Relationship<Child, Parent> for BelongsTo<Child, Parent>
where
    Child: Model + Send + Sync + 'static,
    Parent: Model + Send + Sync + 'static,
{
    fn meta(&self) -> &RelationshipMeta {
        &self.meta
    }

    fn parent(&self) -> &Child {
        &self.child
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }

    fn query(&self) -> QueryBuilder<Parent> {
        let mut query = QueryBuilder::new();
        
        if let Some(foreign_key_value) = self.foreign_key_value() {
            query = query
                .from(&self.meta.related_table)
                .where_eq(&self.meta.local_key, foreign_key_value);
        }
        
        query
    }

    async fn load(&mut self, pool: &Pool<Postgres>) -> ModelResult<()> {
        let results = self.query().get(pool).await?;
        self.parent = results.into_iter().next();
        self.loaded = true;
        Ok(())
    }
}
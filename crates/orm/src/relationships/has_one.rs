//! HasOne Relationship - Clean implementation using new query system

use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;

use crate::error::ModelResult;
use crate::model::Model;
use crate::query::QueryBuilder;

use super::traits::{Relationship, RelationshipMeta};

/// HasOne relationship - parent model has one related model
pub struct HasOne<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    parent: Parent,
    related: Option<Related>,
    meta: RelationshipMeta,
    loaded: bool,
}

impl<Parent, Related> HasOne<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    /// Create a new HasOne relationship
    pub fn new(parent: Parent, foreign_key: &str) -> Self {
        Self {
            parent,
            related: None,
            meta: RelationshipMeta {
                foreign_key: foreign_key.to_string(),
                local_key: Parent::primary_key_name().to_string(),
                related_table: Related::table_name().to_string(),
            },
            loaded: false,
        }
    }

    /// Get the related model if loaded
    pub fn get(&self) -> Option<&Related> {
        self.related.as_ref()
    }

    /// Get the related model as mutable if loaded
    pub fn get_mut(&mut self) -> Option<&mut Related> {
        self.related.as_mut()
    }

    /// Take ownership of the related model
    pub fn take(&mut self) -> Option<Related> {
        self.related.take()
    }

    /// Set the related model
    pub fn set(&mut self, related: Option<Related>) {
        self.related = related;
        self.loaded = true;
    }
}

#[async_trait]
impl<Parent, Related> Relationship<Parent, Related> for HasOne<Parent, Related>
where
    Parent: Model + Send + Sync + 'static,
    Related: Model + Send + Sync + 'static,
{
    fn meta(&self) -> &RelationshipMeta {
        &self.meta
    }

    fn parent(&self) -> &Parent {
        &self.parent
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }

    fn query(&self) -> QueryBuilder<Related> {
        let mut query = QueryBuilder::new();
        
        if let Some(parent_key) = self.parent.primary_key() {
            query = query
                .from(&self.meta.related_table)
                .where_eq(&self.meta.foreign_key, parent_key.to_string());
        }
        
        query
    }

    async fn load(&mut self, pool: &Pool<Postgres>) -> ModelResult<()> {
        let results = self.query().get(pool).await?;
        self.related = results.into_iter().next();
        self.loaded = true;
        Ok(())
    }
}
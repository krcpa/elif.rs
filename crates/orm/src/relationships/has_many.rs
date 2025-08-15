//! HasMany Relationship - Clean implementation using new query system

use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;

use crate::error::ModelResult;
use crate::model::Model;
use crate::query::QueryBuilder;

use super::traits::{Relationship, RelationshipMeta};

/// HasMany relationship - parent model has many related models
pub struct HasMany<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    parent: Parent,
    related: Vec<Related>,
    meta: RelationshipMeta,
    loaded: bool,
}

impl<Parent, Related> HasMany<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    /// Create a new HasMany relationship
    pub fn new(parent: Parent, foreign_key: &str) -> Self {
        Self {
            parent,
            related: Vec::new(),
            meta: RelationshipMeta {
                foreign_key: foreign_key.to_string(),
                local_key: Parent::primary_key_name().to_string(),
                related_table: Related::table_name().to_string(),
            },
            loaded: false,
        }
    }

    /// Get all related models
    pub fn get(&self) -> &[Related] {
        &self.related
    }

    /// Get mutable reference to all related models
    pub fn get_mut(&mut self) -> &mut Vec<Related> {
        &mut self.related
    }

    /// Take ownership of all related models
    pub fn take(&mut self) -> Vec<Related> {
        std::mem::take(&mut self.related)
    }

    /// Set the related models
    pub fn set(&mut self, related: Vec<Related>) {
        self.related = related;
        self.loaded = true;
    }

    /// Add a related model
    pub fn push(&mut self, related: Related) {
        self.related.push(related);
    }

    /// Get the count of related models
    pub fn len(&self) -> usize {
        self.related.len()
    }

    /// Check if there are any related models
    pub fn is_empty(&self) -> bool {
        self.related.is_empty()
    }

    /// Iterate over related models
    pub fn iter(&self) -> std::slice::Iter<Related> {
        self.related.iter()
    }

    /// Iterate mutably over related models
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Related> {
        self.related.iter_mut()
    }
}

#[async_trait]
impl<Parent, Related> Relationship<Parent, Related> for HasMany<Parent, Related>
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
        self.related = results;
        self.loaded = true;
        Ok(())
    }
}

// Iterator implementations for convenience
impl<Parent, Related> IntoIterator for HasMany<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    type Item = Related;
    type IntoIter = std::vec::IntoIter<Related>;

    fn into_iter(self) -> Self::IntoIter {
        self.related.into_iter()
    }
}

impl<'a, Parent, Related> IntoIterator for &'a HasMany<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    type Item = &'a Related;
    type IntoIter = std::slice::Iter<'a, Related>;

    fn into_iter(self) -> Self::IntoIter {
        self.related.iter()
    }
}

impl<'a, Parent, Related> IntoIterator for &'a mut HasMany<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    type Item = &'a mut Related;
    type IntoIter = std::slice::IterMut<'a, Related>;

    fn into_iter(self) -> Self::IntoIter {
        self.related.iter_mut()
    }
}
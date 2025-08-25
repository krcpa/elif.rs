//! Relationship Traits - Core traits for relationship management

use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;

use crate::error::ModelResult;
use crate::model::Model;
use crate::query::QueryBuilder;

/// Relationship metadata
#[derive(Debug, Clone)]
pub struct RelationshipMeta {
    pub foreign_key: String,
    pub local_key: String,
    pub related_table: String,
}

/// Core relationship trait - simplified and clean
#[async_trait]
pub trait Relationship<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    /// Get relationship metadata
    fn meta(&self) -> &RelationshipMeta;

    /// Get the parent model instance
    fn parent(&self) -> &Parent;

    /// Check if the relationship has been loaded
    fn is_loaded(&self) -> bool;

    /// Mark the relationship as loaded
    fn set_loaded(&mut self, loaded: bool);

    /// Build a query for this relationship
    fn query(&self) -> QueryBuilder<Related>;

    /// Load the relationship from the database
    async fn load(&mut self, pool: &Pool<Postgres>) -> ModelResult<()>;
}

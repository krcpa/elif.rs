//! Relationship Loading Traits - Type-safe loading interfaces
//!
//! Provides traits for loading relationships with proper type safety
//! and support for both single instance and batch loading.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres};

use crate::error::ModelResult;
use crate::model::Model;

/// Trait for loading relationships with type safety
#[async_trait]
pub trait TypeSafeRelationshipLoader<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync + DeserializeOwned,
{
    /// Load a single relationship instance
    async fn load_for_instance(
        &self,
        parent: &Parent,
        pool: &Pool<Postgres>,
    ) -> ModelResult<Related>;

    /// Load relationship for multiple parents (eager loading)
    async fn load_for_instances(
        &self,
        parents: &mut [Parent],
        pool: &Pool<Postgres>,
    ) -> ModelResult<Vec<Related>>;

    /// Load with specific type conversion
    async fn load_typed<T>(&self, parent: &Parent, pool: &Pool<Postgres>) -> ModelResult<T>
    where
        T: DeserializeOwned + Send + Sync;
}

//! Controller System - Base controller functionality with CRUD operations
//! 
//! Provides the Controller trait and base functionality for handling HTTP requests
//! with database integration via the ORM system.

use std::sync::Arc;
use axum::{
    extract::{State, Path, Query},
    response::{Json, Response, IntoResponse},
    http::StatusCode,
};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use elif_core::Container;
use elif_orm::{Model};
use crate::{HttpResult, HttpError, ApiResponse};

/// Query parameters for pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(20),
            sort: Some("id".to_string()),
            order: Some("asc".to_string()),
        }
    }
}

/// Controller trait with standard CRUD operations
#[axum::async_trait]
pub trait Controller<T>
where
    T: Model + Send + Sync + Serialize + DeserializeOwned + 'static,
    T::PrimaryKey: Send + Sync + Serialize + DeserializeOwned,
{
    /// Get the database pool for this controller
    async fn get_pool(&self, container: &Container) -> HttpResult<Arc<Pool<Postgres>>>;

    /// List all entities with optional pagination
    async fn index(
        &self,
        State(container): State<Arc<Container>>,
        Query(params): Query<PaginationParams>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let models = T::all(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(models);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Get a single entity by ID
    async fn show(
        &self,
        State(container): State<Arc<Container>>,
        Path(id): Path<T::PrimaryKey>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let model = T::find(&pool, id)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Resource not found"))?;

        let api_response = ApiResponse::success(model);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Create a new entity
    async fn create(
        &self,
        State(container): State<Arc<Container>>,
        Json(model): Json<T>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let created_model = T::create(&pool, model)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(created_model);
        Ok((StatusCode::CREATED, Json(api_response)).into_response())
    }

    /// Update an existing entity
    async fn update(
        &self,
        State(container): State<Arc<Container>>,
        Path(id): Path<T::PrimaryKey>,
        Json(update_data): Json<Value>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let mut model = T::find(&pool, id)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Resource not found"))?;

        // Apply updates to model (simplified - in real implementation would use proper merging)
        // This would need model-specific update logic
        model.update(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(model);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Delete an entity
    async fn destroy(
        &self,
        State(container): State<Arc<Container>>,
        Path(id): Path<T::PrimaryKey>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let model = T::find(&pool, id)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?
            .ok_or_else(|| HttpError::not_found("Resource not found"))?;

        model.delete(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(serde_json::json!({"deleted": true}));
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }
}

/// Base controller implementation with database pool
pub struct BaseController {
    /// Direct database pool (simplified approach)
    pool: Option<Arc<Pool<Postgres>>>,
}

impl BaseController {
    pub fn new() -> Self {
        Self {
            pool: None,
        }
    }

    pub fn with_pool(pool: Arc<Pool<Postgres>>) -> Self {
        Self {
            pool: Some(pool),
        }
    }
}

#[axum::async_trait]
impl<T> Controller<T> for BaseController
where
    T: Model + Send + Sync + Serialize + DeserializeOwned + 'static,
    T::PrimaryKey: Send + Sync + Serialize + DeserializeOwned,
{
    async fn get_pool(&self, _container: &Container) -> HttpResult<Arc<Pool<Postgres>>> {
        self.pool.clone().ok_or_else(|| 
            HttpError::internal_server_error("Database pool not configured for this controller")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    // Mock model for testing
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct TestUser {
        pub id: Uuid,
        pub name: String,
        pub email: String,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    impl Model for TestUser {
        type PrimaryKey = Uuid;

        fn table_name() -> &'static str {
            "users"
        }

        fn primary_key(&self) -> Option<Self::PrimaryKey> {
            Some(self.id)
        }

        fn set_primary_key(&mut self, key: Self::PrimaryKey) {
            self.id = key;
        }

        fn from_row(row: &sqlx::postgres::PgRow) -> elif_orm::ModelResult<Self> {
            Ok(TestUser {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                email: row.try_get("email")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        }

        fn to_fields(&self) -> HashMap<String, serde_json::Value> {
            let mut fields = HashMap::new();
            fields.insert("name".to_string(), serde_json::json!(self.name));
            fields.insert("email".to_string(), serde_json::json!(self.email));
            fields
        }
    }

    #[tokio::test]
    async fn test_base_controller_creation() {
        let controller = BaseController::new();
        assert!(controller.pool.is_none());
    }

    #[tokio::test]
    async fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, Some(1));
        assert_eq!(params.per_page, Some(20));
        assert_eq!(params.sort, Some("id".to_string()));
        assert_eq!(params.order, Some("asc".to_string()));
    }
}
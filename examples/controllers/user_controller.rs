//! User Controller Example - Full CRUD operations with validation and error handling
//! 
//! Demonstrates how to implement a controller using the elif-http Controller trait
//! with custom validation, error handling, and business logic.

use std::sync::Arc;
use axum::{
    extract::{State, Path, Query},
    response::Json,
    http::StatusCode,
};
use serde_json::Value;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use elif_core::Container;
use elif_http::{
    Controller, BaseController, PaginationParams,
    HttpResult, HttpError, ApiResponse
};
use axum::response::{Json, IntoResponse, Response};

use super::user_model::{User, CreateUserRequest, UpdateUserRequest};

/// UserController with custom business logic
pub struct UserController {
    base: BaseController,
}

impl UserController {
    pub fn new() -> Self {
        Self {
            base: BaseController::new(),
        }
    }

    pub fn with_pool(pool: Arc<Pool<Postgres>>) -> Self {
        Self {
            base: BaseController::with_pool(pool),
        }
    }

    /// Custom method: Get active users only
    pub async fn active_users(
        &self,
        State(container): State<Arc<Container>>,
        Query(params): Query<PaginationParams>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        // Use the ORM query builder for custom filtering
        let users = User::query()
            .select("*")
            .where_clause("is_active = $1")
            .order_by("created_at DESC")
            .limit(params.per_page.unwrap_or(20) as i32)
            .execute(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(users);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Custom method: Search users by name or email
    pub async fn search(
        &self,
        State(container): State<Arc<Container>>,
        Query(query): Query<SearchParams>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let search_term = query.q.unwrap_or_default();
        if search_term.len() < 3 {
            return Err(HttpError::bad_request("Search term must be at least 3 characters"));
        }

        let users = User::query()
            .select("*")
            .where_clause("name ILIKE $1 OR email ILIKE $1")
            .order_by("name ASC")
            .limit(50)
            .execute_with_params(&pool, vec![format!("%{}%", search_term)])
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(users);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Custom method: Get user statistics
    pub async fn statistics(
        &self,
        State(container): State<Arc<Container>>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        let total_count = User::count(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let active_count = User::query()
            .select("COUNT(*)")
            .where_clause("is_active = $1")
            .execute_scalar(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let stats = UserStats {
            total_users: total_count,
            active_users: active_count,
            inactive_users: total_count - active_count,
        };

        let api_response = ApiResponse::success(stats);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }
}

/// Implement the Controller trait for UserController
#[axum::async_trait]
impl Controller<User> for UserController {
    async fn get_pool(&self, container: &Container) -> HttpResult<Arc<Pool<Postgres>>> {
        self.base.get_pool(container).await
    }

    /// Override create method with validation
    async fn create(
        &self,
        State(container): State<Arc<Container>>,
        Json(create_request): Json<CreateUserRequest>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        // Convert DTO to model
        let mut user = create_request.into_user();
        
        // Validate the user
        user.validate()
            .map_err(|e| HttpError::validation_error(&e))?;

        // Check for duplicate email
        let existing_user = User::query()
            .select("id")
            .where_clause("email = $1")
            .execute_first(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        if existing_user.is_some() {
            return Err(HttpError::conflict("User with this email already exists"));
        }

        // Create the user
        let created_user = User::create(&pool, user)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(created_user);
        Ok((StatusCode::CREATED, Json(api_response)).into_response())
    }

    /// Override update method with validation and partial updates
    async fn update(
        &self,
        State(container): State<Arc<Container>>,
        Path(id): Path<Uuid>,
        Json(update_request): Json<UpdateUserRequest>,
    ) -> HttpResult<Response> {
        let pool = self.get_pool(&container).await?;
        
        // Find existing user
        let mut user = User::find(&pool, id)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?
            .ok_or_else(|| HttpError::not_found("User not found"))?;

        // Apply updates
        update_request.apply_to_user(&mut user);

        // Validate updated user
        user.validate()
            .map_err(|e| HttpError::validation_error(&e))?;

        // Check for email conflicts (if email was changed)
        if let Some(email) = &update_request.email {
            let existing_user = User::query()
                .select("id")
                .where_clause("email = $1 AND id != $2")
                .execute_first_with_params(&pool, vec![email.clone(), id.to_string()])
                .await
                .map_err(|e| HttpError::database_error(&e.to_string()))?;

            if existing_user.is_some() {
                return Err(HttpError::conflict("Another user with this email already exists"));
            }
        }

        // Update the user
        user.update(&pool)
            .await
            .map_err(|e| HttpError::database_error(&e.to_string()))?;

        let api_response = ApiResponse::success(user);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }
}

/// Search parameters for user search
#[derive(Debug, serde::Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
}

/// User statistics response
#[derive(Debug, serde::Serialize)]
pub struct UserStats {
    pub total_users: i64,
    pub active_users: i64,
    pub inactive_users: i64,
}

/// Route setup helper for UserController
pub fn setup_user_routes(router: axum::Router<Arc<Container>>) -> axum::Router<Arc<Container>> {
    let user_controller = Arc::new(UserController::new());

    router
        // Standard CRUD routes
        .route("/users", axum::routing::get({
            let controller = Arc::clone(&user_controller);
            move |state, query| {
                let controller = Arc::clone(&controller);
                async move { controller.index(state, query).await }
            }
        }))
        .route("/users", axum::routing::post({
            let controller = Arc::clone(&user_controller);
            move |state, json| {
                let controller = Arc::clone(&controller);
                async move { controller.create(state, json).await }
            }
        }))
        .route("/users/:id", axum::routing::get({
            let controller = Arc::clone(&user_controller);
            move |state, path| {
                let controller = Arc::clone(&controller);
                async move { controller.show(state, path).await }
            }
        }))
        .route("/users/:id", axum::routing::put({
            let controller = Arc::clone(&user_controller);
            move |state, path, json| {
                let controller = Arc::clone(&controller);
                async move { controller.update(state, path, json).await }
            }
        }))
        .route("/users/:id", axum::routing::delete({
            let controller = Arc::clone(&user_controller);
            move |state, path| {
                let controller = Arc::clone(&controller);
                async move { controller.destroy(state, path).await }
            }
        }))
        // Custom routes
        .route("/users/active", axum::routing::get({
            let controller = Arc::clone(&user_controller);
            move |state, query| {
                let controller = Arc::clone(&controller);
                async move { controller.active_users(state, query).await }
            }
        }))
        .route("/users/search", axum::routing::get({
            let controller = Arc::clone(&user_controller);
            move |state, query| {
                let controller = Arc::clone(&controller);
                async move { controller.search(state, query).await }
            }
        }))
        .route("/users/statistics", axum::routing::get({
            let controller = Arc::clone(&user_controller);
            move |state| {
                let controller = Arc::clone(&controller);
                async move { controller.statistics(state).await }
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_controller_creation() {
        let controller = UserController::new();
        assert!(controller.base.pool.is_none());
    }

    #[test]
    fn test_search_params() {
        let params = SearchParams { q: Some("test".to_string()) };
        assert_eq!(params.q.unwrap(), "test");
    }

    #[test]
    fn test_user_stats() {
        let stats = UserStats {
            total_users: 100,
            active_users: 80,
            inactive_users: 20,
        };
        assert_eq!(stats.total_users, 100);
        assert_eq!(stats.active_users, 80);
        assert_eq!(stats.inactive_users, 20);
    }
}
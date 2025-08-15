//! Service-Oriented Controller System
//! 
//! Provides a clean separation between HTTP handling (controllers) and business logic (services).
//! Controllers are thin HTTP handlers that delegate to injected services.

use std::{sync::Arc, pin::Pin, future::Future};
use axum::{
    extract::{State, Path, Query},
    response::{Json, Response, IntoResponse},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use elif_core::Container;
use crate::{HttpResult, HttpError, ApiResponse};

/// Query parameters for pagination and filtering
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub filter: Option<String>,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(20),
            sort: Some("id".to_string()),
            order: Some("asc".to_string()),
            filter: None,
        }
    }
}

/// Pagination metadata for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: Option<u64>,
    pub total_pages: Option<u32>,
    pub has_more: bool,
}

/// Base controller providing HTTP utilities (no infrastructure dependencies)
#[derive(Clone)]
pub struct BaseController;

impl BaseController {
    pub fn new() -> Self {
        Self
    }

    /// Validate and normalize pagination parameters
    pub fn normalize_pagination(&self, params: &QueryParams) -> (u32, u32, u64) {
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).min(100).max(1);
        let offset = (page - 1) * per_page;
        (page, per_page, offset as u64)
    }

    /// Create standardized success response
    pub fn success_response<T: Serialize>(&self, data: T) -> HttpResult<Response> {
        let api_response = ApiResponse::success(data);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Create standardized created response
    pub fn created_response<T: Serialize>(&self, data: T) -> HttpResult<Response> {
        let api_response = ApiResponse::success(data);
        Ok((StatusCode::CREATED, Json(api_response)).into_response())
    }

    /// Create paginated response with metadata
    pub fn paginated_response<T: Serialize>(&self, data: Vec<T>, meta: PaginationMeta) -> HttpResult<Response> {
        let response_data = serde_json::json!({
            "data": data,
            "meta": meta
        });
        let api_response = ApiResponse::success(response_data);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }

    /// Create standardized delete response
    pub fn deleted_response<T: Serialize>(&self, resource_name: &str, deleted_id: Option<T>) -> HttpResult<Response> {
        let mut response_data = serde_json::json!({
            "message": format!("{} deleted successfully", resource_name)
        });
        
        if let Some(id) = deleted_id {
            response_data["deleted_id"] = serde_json::to_value(id)?;
        }
        
        let api_response = ApiResponse::success(response_data);
        Ok((StatusCode::OK, Json(api_response)).into_response())
    }
}

/// Send-safe controller trait for HTTP request handling
/// Controllers delegate business logic to injected services
pub trait Controller: Send + Sync {
    /// List resources with pagination
    fn index(
        &self,
        container: State<Arc<Container>>,
        params: Query<QueryParams>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<Response>> + Send>>;

    /// Get single resource by ID
    fn show(
        &self,
        container: State<Arc<Container>>,
        id: Path<String>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<Response>> + Send>>;

    /// Create new resource
    fn create(
        &self,
        container: State<Arc<Container>>,
        data: Json<Value>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<Response>> + Send>>;

    /// Update existing resource
    fn update(
        &self,
        container: State<Arc<Container>>,
        id: Path<String>,
        data: Json<Value>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<Response>> + Send>>;

    /// Delete resource
    fn destroy(
        &self,
        container: State<Arc<Container>>,
        id: Path<String>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<Response>> + Send>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_base_controller_creation() {
        let _controller = BaseController::new();
    }

    #[tokio::test]
    async fn test_pagination_normalization() {
        let controller = BaseController::new();
        let params = QueryParams {
            page: Some(5),
            per_page: Some(10),
            ..Default::default()
        };
        
        let (page, per_page, offset) = controller.normalize_pagination(&params);
        assert_eq!(page, 5);
        assert_eq!(per_page, 10);
        assert_eq!(offset, 40);
    }

    #[tokio::test]
    async fn test_pagination_limits() {
        let controller = BaseController::new();
        let params = QueryParams {
            page: Some(0),
            per_page: Some(200),
            ..Default::default()
        };
        
        let (page, per_page, offset) = controller.normalize_pagination(&params);
        assert_eq!(page, 1);
        assert_eq!(per_page, 100);
        assert_eq!(offset, 0);
    }

    #[tokio::test]
    async fn test_success_response_creation() {
        let controller = BaseController::new();
        let data = json!({"message": "test"});
        let response = controller.success_response(data);
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_pagination_meta_creation() {
        let meta = PaginationMeta {
            page: 1,
            per_page: 20,
            total: Some(100),
            total_pages: Some(5),
            has_more: true,
        };
        
        assert_eq!(meta.page, 1);
        assert_eq!(meta.per_page, 20);
        assert_eq!(meta.total, Some(100));
    }
}
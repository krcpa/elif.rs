//! Controller System for Route Organization
//! 
//! Provides both service-oriented controllers and a new ElifController system
//! for automatic route registration and organization.

use std::{sync::Arc, pin::Pin, future::Future};
use crate::{
    request::{ElifState, ElifPath, ElifQuery, ElifRequest},
    response::{ElifJson, ElifResponse},
    routing::{HttpMethod, params::ParamType},
};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use elif_core::Container;
use crate::{HttpResult, response::ApiResponse};

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
    pub fn success_response<T: Serialize>(&self, data: T) -> HttpResult<ElifResponse> {
        let api_response = ApiResponse::success(data);
        Ok(ElifResponse::ok().json(&api_response)?)
    }

    /// Create standardized created response
    pub fn created_response<T: Serialize>(&self, data: T) -> HttpResult<ElifResponse> {
        let api_response = ApiResponse::success(data);
        Ok(ElifResponse::created().json(&api_response)?)
    }

    /// Create paginated response with metadata
    pub fn paginated_response<T: Serialize>(&self, data: Vec<T>, meta: PaginationMeta) -> HttpResult<ElifResponse> {
        let response_data = serde_json::json!({
            "data": data,
            "meta": meta
        });
        Ok(ElifResponse::ok().json(&response_data)?)
    }

    /// Create standardized delete response
    pub fn deleted_response<T: Serialize>(&self, resource_name: &str, deleted_id: Option<T>) -> HttpResult<ElifResponse> {
        let mut response_data = serde_json::json!({
            "message": format!("{} deleted successfully", resource_name)
        });
        
        if let Some(id) = deleted_id {
            response_data["deleted_id"] = serde_json::to_value(id)?;
        }
        
        let api_response = ApiResponse::success(response_data);
        Ok(ElifResponse::ok().json(&api_response)?)
    }
}

/// Send-safe controller trait for HTTP request handling
/// Controllers delegate business logic to injected services
pub trait Controller: Send + Sync {
    /// List resources with pagination
    fn index(
        &self,
        container: ElifState<Arc<Container>>,
        params: ElifQuery<QueryParams>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;

    /// Get single resource by ID
    fn show(
        &self,
        container: ElifState<Arc<Container>>,
        id: ElifPath<String>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;

    /// Create new resource
    fn create(
        &self,
        container: ElifState<Arc<Container>>,
        data: ElifJson<Value>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;

    /// Update existing resource
    fn update(
        &self,
        container: ElifState<Arc<Container>>,
        id: ElifPath<String>,
        data: ElifJson<Value>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;

    /// Delete resource
    fn destroy(
        &self,
        container: ElifState<Arc<Container>>,
        id: ElifPath<String>,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;
}

/// Route parameter definition for controllers
#[derive(Debug, Clone)]
pub struct RouteParam {
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    pub default: Option<String>,
}

impl RouteParam {
    pub fn new(name: &str, param_type: ParamType) -> Self {
        Self {
            name: name.to_string(),
            param_type,
            required: true,
            default: None,
        }
    }
    
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
    
    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self.required = false;
        self
    }
}

/// Controller route definition
#[derive(Debug, Clone)]
pub struct ControllerRoute {
    pub method: HttpMethod,
    pub path: String,
    pub handler_name: String,
    pub middleware: Vec<String>,
    pub params: Vec<RouteParam>,
}

impl ControllerRoute {
    pub fn new(method: HttpMethod, path: &str, handler_name: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            handler_name: handler_name.to_string(),
            middleware: vec![],
            params: vec![],
        }
    }
    
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
    
    pub fn with_params(mut self, params: Vec<RouteParam>) -> Self {
        self.params = params;
        self
    }
    
    pub fn add_param(mut self, param: RouteParam) -> Self {
        self.params.push(param);
        self
    }
}

/// Main trait for controllers with automatic route registration
pub trait ElifController: Send + Sync + 'static {
    /// Controller name for identification
    fn name(&self) -> &str;
    
    /// Base path for all routes in this controller
    fn base_path(&self) -> &str;
    
    /// Route definitions for this controller
    fn routes(&self) -> Vec<ControllerRoute>;
    
    /// Dependencies required by this controller (optional)
    fn dependencies(&self) -> Vec<String> { 
        vec![] 
    }
    
    /// Handle a request by dispatching to the appropriate method
    fn handle_request(
        &self,
        method_name: String,
        request: ElifRequest,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>>;
}

/// Macro to help implement controller method dispatch
#[macro_export]
macro_rules! controller_dispatch {
    ($self:expr, $method_name:expr, $request:expr, {
        $($method:literal => $handler:expr),*
    }) => {
        match $method_name {
            $($method => Box::pin($handler($self, $request)),)*
            _ => Box::pin(async move {
                use crate::response::ElifResponse;
                Ok(ElifResponse::not_found().text(&format!("Handler '{}' not found", $method_name)))
            })
        }
    };
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
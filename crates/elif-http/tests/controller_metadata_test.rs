//! Test for controller metadata extraction from type registry

use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http::controller::{ElifController, ControllerRoute};
use elif_http::routing::HttpMethod;
use elif_http::bootstrap::ControllerRegistry;
use elif_core::modules::CompileTimeModuleMetadata;
use elif_core::container::IocContainer;
use std::sync::Arc;

#[derive(Debug)]
pub struct MetadataTestController;

impl MetadataTestController {
    pub fn new() -> Self {
        Self
    }
}

// Manual implementation of ElifController for testing
#[async_trait::async_trait]
impl ElifController for MetadataTestController {
    fn name(&self) -> &str {
        "MetadataTestController"
    }

    fn base_path(&self) -> &str {
        "/metadata-test"
    }

    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            ControllerRoute {
                method: HttpMethod::GET,
                path: "".to_string(),
                handler_name: "index".to_string(),
                middleware: vec!["auth".to_string()],
                params: vec![],
            },
            ControllerRoute {
                method: HttpMethod::POST,
                path: "/create".to_string(),
                handler_name: "create".to_string(),
                middleware: vec!["validate".to_string(), "auth".to_string()],
                params: vec![],
            },
        ]
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string(), "logger".to_string()]
    }

    async fn handle_request(
        self: std::sync::Arc<Self>,
        method_name: String,
        _request: ElifRequest,
    ) -> HttpResult<ElifResponse> {
        match method_name.as_str() {
            "index" => Ok(ElifResponse::ok().text("Metadata test index")),
            "create" => Ok(ElifResponse::created().text("Created resource")),
            _ => Ok(ElifResponse::not_found().text(&format!("Handler '{}' not found", method_name))),
        }
    }
}

#[tokio::test]
async fn test_controller_metadata_extraction() {
    // Register the controller in the type registry
    elif_http::bootstrap::register_controller_type(
        "MetadataTestController",
        || Box::new(MetadataTestController::new())
    );
    
    // Create a test module with the controller
    let module = CompileTimeModuleMetadata::new("TestModule".to_string())
        .with_controller("MetadataTestController".to_string());
    
    let modules = vec![module];
    let container = Arc::new(IocContainer::new());
    
    // Create controller registry and extract metadata
    let registry = ControllerRegistry::from_modules(&modules, container).expect("Should create registry");
    
    // Verify the controller metadata was properly extracted
    let controller_names = registry.get_controller_names();
    assert!(controller_names.contains(&"MetadataTestController".to_string()));
    
    let metadata = registry.get_controller_metadata("MetadataTestController").expect("Should have metadata");
    
    // Verify basic metadata
    assert_eq!(metadata.name, "MetadataTestController");
    assert_eq!(metadata.base_path, "/metadata-test");
    assert_eq!(metadata.routes.len(), 2);
    assert_eq!(metadata.dependencies.len(), 2);
    assert!(metadata.dependencies.contains(&"database".to_string()));
    assert!(metadata.dependencies.contains(&"logger".to_string()));
    
    // Verify route metadata
    let index_route = &metadata.routes[0];
    assert_eq!(index_route.method, HttpMethod::GET);
    assert_eq!(index_route.path, "");
    assert_eq!(index_route.handler_name, "index");
    assert_eq!(index_route.middleware.len(), 1);
    assert!(index_route.middleware.contains(&"auth".to_string()));
    
    let create_route = &metadata.routes[1];
    assert_eq!(create_route.method, HttpMethod::POST);
    assert_eq!(create_route.path, "/create");
    assert_eq!(create_route.handler_name, "create");
    assert_eq!(create_route.middleware.len(), 2);
    assert!(create_route.middleware.contains(&"validate".to_string()));
    assert!(create_route.middleware.contains(&"auth".to_string()));
}

#[test]
fn test_controller_route_validation() {
    // Register controllers for conflict testing
    elif_http::bootstrap::register_controller_type(
        "MetadataTestController_V1",
        || Box::new(MetadataTestController::new())
    );
    
    // Create modules
    let module = CompileTimeModuleMetadata::new("TestModule".to_string())
        .with_controller("MetadataTestController_V1".to_string());
    
    let modules = vec![module];
    let container = Arc::new(IocContainer::new());
    
    // Create registry and validate routes
    let registry = ControllerRegistry::from_modules(&modules, container).expect("Should create registry");
    
    // Should not have conflicts with single controller
    let validation_result = registry.validate_routes();
    assert!(validation_result.is_ok(), "Single controller should not have route conflicts");
    
    // Verify total route count
    assert_eq!(registry.total_routes(), 2);
}
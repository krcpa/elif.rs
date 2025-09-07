//! End-to-end integration test for Controller Auto-Registration Phase 2
//! 
//! This test verifies that the complete controller auto-registration system works
//! from module discovery through controller metadata extraction and validation.

use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http::controller::{ElifController, ControllerRoute};
use elif_http::routing::HttpMethod;
use elif_http::bootstrap::{AppBootstrapper, ControllerRegistry};
use elif_core::modules::{CompileTimeModuleMetadata, register_module_globally};
use elif_core::container::IocContainer;
use std::sync::Arc;

// Test controllers for integration testing
#[derive(Debug)]
pub struct UserController;

impl UserController {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ElifController for UserController {
    fn name(&self) -> &str {
        "UserController"
    }

    fn base_path(&self) -> &str {
        "/api/users"
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
                method: HttpMethod::GET,
                path: "/{id}".to_string(),
                handler_name: "show".to_string(),
                middleware: vec!["auth".to_string()],
                params: vec![],
            },
            ControllerRoute {
                method: HttpMethod::POST,
                path: "".to_string(),
                handler_name: "create".to_string(),
                middleware: vec!["auth".to_string(), "validate".to_string()],
                params: vec![],
            },
        ]
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["user_service".to_string(), "logger".to_string()]
    }

    async fn handle_request(
        self: Arc<Self>,
        method_name: String,
        _request: ElifRequest,
    ) -> HttpResult<ElifResponse> {
        match method_name.as_str() {
            "index" => Ok(ElifResponse::ok().json(&vec!["user1", "user2"]).unwrap()),
            "show" => Ok(ElifResponse::ok().json(&"user details").unwrap()),
            "create" => Ok(ElifResponse::created().json(&"user created").unwrap()),
            _ => Ok(ElifResponse::not_found().text(&format!("Handler '{}' not found", method_name))),
        }
    }
}

#[derive(Debug)]
pub struct PaymentController;

impl PaymentController {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ElifController for PaymentController {
    fn name(&self) -> &str {
        "PaymentController"
    }

    fn base_path(&self) -> &str {
        "/api/payments"
    }

    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            ControllerRoute {
                method: HttpMethod::POST,
                path: "".to_string(),
                handler_name: "create".to_string(),
                middleware: vec!["auth".to_string(), "rate_limit".to_string()],
                params: vec![],
            },
            ControllerRoute {
                method: HttpMethod::GET,
                path: "/{payment_id}/status".to_string(),
                handler_name: "status".to_string(),
                middleware: vec!["auth".to_string()],
                params: vec![],
            },
        ]
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["payment_service".to_string(), "audit_logger".to_string()]
    }

    async fn handle_request(
        self: Arc<Self>,
        method_name: String,
        _request: ElifRequest,
    ) -> HttpResult<ElifResponse> {
        match method_name.as_str() {
            "create" => Ok(ElifResponse::created().json(&"payment created").unwrap()),
            "status" => Ok(ElifResponse::ok().json(&"payment status").unwrap()),
            _ => Ok(ElifResponse::not_found().text(&format!("Handler '{}' not found", method_name))),
        }
    }
}

#[tokio::test]
async fn test_phase2_controller_auto_registration_complete() {
    // Use unique names to avoid conflicts with other tests
    let user_controller_name = "UserController_Phase2";
    let payment_controller_name = "PaymentController_Phase2";
    
    // Register controllers in the type registry (simulating macro auto-registration)
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(user_controller_name) {
        elif_http::bootstrap::register_controller_type(
            user_controller_name,
            || Box::new(UserController::new())
        );
    }
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(payment_controller_name) {
        elif_http::bootstrap::register_controller_type(
            payment_controller_name, 
            || Box::new(PaymentController::new())
        );
    }
    
    // Test 1: Controller registry creation and metadata extraction
    let modules = vec![
        CompileTimeModuleMetadata::new("Phase2TestModule".to_string())
            .with_controller(user_controller_name.to_string())
            .with_controller(payment_controller_name.to_string())
    ];
    let container = Arc::new(IocContainer::new());
    
    let registry = ControllerRegistry::from_modules(&modules, container)
        .expect("Should create controller registry");
    
    // Verify both controllers were discovered
    let controller_names = registry.get_controller_names();
    assert_eq!(controller_names.len(), 2);
    assert!(controller_names.contains(&user_controller_name.to_string()));
    assert!(controller_names.contains(&payment_controller_name.to_string()));
    
    // Test 2: Metadata extraction verification
    let user_metadata = registry.get_controller_metadata(user_controller_name)
        .expect("Should have UserController metadata");
    assert_eq!(user_metadata.name, "UserController");
    assert_eq!(user_metadata.base_path, "/api/users");
    assert_eq!(user_metadata.routes.len(), 3);
    assert_eq!(user_metadata.dependencies.len(), 2);
    
    let payment_metadata = registry.get_controller_metadata(payment_controller_name)
        .expect("Should have PaymentController metadata");
    assert_eq!(payment_metadata.name, "PaymentController");
    assert_eq!(payment_metadata.base_path, "/api/payments");
    assert_eq!(payment_metadata.routes.len(), 2);
    assert_eq!(payment_metadata.dependencies.len(), 2);
    
    // Test 3: Route conflict validation
    let validation_result = registry.validate_routes();
    assert!(validation_result.is_ok(), "Should not have route conflicts with different base paths");
    
    // Test 4: Total route counting
    assert_eq!(registry.total_routes(), 5); // 3 user routes + 2 payment routes
    
    // Test 5: Controller creation from type registry
    let user_controller = elif_http::bootstrap::create_controller(user_controller_name)
        .expect("Should create UserController instance");
    assert_eq!(user_controller.name(), "UserController");
    assert_eq!(user_controller.routes().len(), 3);
    
    let payment_controller = elif_http::bootstrap::create_controller(payment_controller_name)
        .expect("Should create PaymentController instance");
    assert_eq!(payment_controller.name(), "PaymentController");
    assert_eq!(payment_controller.routes().len(), 2);
}

#[tokio::test]
async fn test_bootstrap_integration_with_controllers() {
    // Use unique controller name for bootstrap test
    let bootstrap_controller_name = "UserController_Bootstrap";
    
    // Ensure controllers are registered
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(bootstrap_controller_name) {
        elif_http::bootstrap::register_controller_type(
            bootstrap_controller_name,
            || Box::new(UserController::new())
        );
    }
    
    // Register a module for bootstrap testing
    let bootstrap_module = CompileTimeModuleMetadata::new("BootstrapTestModule".to_string())
        .with_controller(bootstrap_controller_name.to_string())
        .with_provider("user_service".to_string());
    
    register_module_globally(bootstrap_module);
    
    // Test the full bootstrap pipeline (without actually starting the server)
    let bootstrapper = AppBootstrapper::new()
        .expect("Should create bootstrapper");
    
    // Verify the module was discovered
    let modules = bootstrapper.modules();
    let has_bootstrap_module = modules.iter()
        .any(|m| m.name == "BootstrapTestModule" || m.controllers.contains(&bootstrap_controller_name.to_string()));
    assert!(has_bootstrap_module, "Bootstrap should discover module with controller");
    
    // Test load order calculation
    let load_order = bootstrapper.load_order();
    assert!(!load_order.is_empty(), "Should have calculated load order");
}

#[test]
fn test_controller_type_registry_status() {
    // Use unique controller name for status test
    let status_controller_name = "UserController_Status";
    
    // Ensure at least one controller is registered for this test
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(status_controller_name) {
        elif_http::bootstrap::register_controller_type(
            status_controller_name,
            || Box::new(UserController::new())
        );
    }
    
    // Verify that the global type registry contains controllers
    let registered_types = elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.get_registered_types();
    
    // Should have at least the controller we just registered
    assert!(registered_types.len() >= 1, "Should have at least one registered controller type");
    
    // Print status for debugging
    println!("Registered controller types ({}):", registered_types.len());
    for controller_type in &registered_types {
        println!("  - {}", controller_type);
    }
    
    // Verify registry is functional
    assert!(elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.count() >= 1);
    assert!(elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(status_controller_name));
}
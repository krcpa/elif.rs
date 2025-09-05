//! Test for controller auto-registration functionality

use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http::controller::{ElifController, ControllerRoute};
use elif_http::routing::HttpMethod;

#[derive(Debug)]
pub struct AutoRegTestController;

impl AutoRegTestController {
    pub fn new() -> Self {
        Self
    }
}

// Manual implementation of ElifController for testing
#[async_trait::async_trait]
impl ElifController for AutoRegTestController {
    fn name(&self) -> &str {
        "AutoRegTestController"
    }

    fn base_path(&self) -> &str {
        "/auto-test"
    }

    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            ControllerRoute {
                method: HttpMethod::GET,
                path: "".to_string(),
                handler_name: "index".to_string(),
                middleware: vec![],
                params: vec![],
            },
            ControllerRoute {
                method: HttpMethod::GET,
                path: "/info".to_string(),
                handler_name: "info".to_string(),
                middleware: vec![],
                params: vec![],
            },
        ]
    }

    fn dependencies(&self) -> Vec<String> {
        vec![]
    }

    async fn handle_request(
        self: std::sync::Arc<Self>,
        method_name: String,
        _request: ElifRequest,
    ) -> HttpResult<ElifResponse> {
        match method_name.as_str() {
            "index" => Ok(ElifResponse::ok().text("Auto-registered controller works!")),
            "info" => Ok(ElifResponse::ok().text("Controller info endpoint")),
            _ => Ok(ElifResponse::not_found().text(&format!("Handler '{}' not found", method_name))),
        }
    }
}

#[tokio::test]
async fn test_controller_manual_registration() {
    // Use a unique name for this test to avoid conflicts
    let controller_name = "AutoRegTestController_Async";
    
    // Check if already registered to avoid duplicate registration
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(controller_name) {
        // Manually register the controller to test the registry system
        elif_http::bootstrap::register_controller_type(
            controller_name,
            || Box::new(AutoRegTestController::new())
        );
    }
    
    // Test that the controller can be created from the registry
    let result = elif_http::bootstrap::create_controller(controller_name);
    
    match result {
        Ok(controller) => {
            assert_eq!(controller.name(), "AutoRegTestController");
            assert_eq!(controller.base_path(), "/auto-test");
            assert_eq!(controller.routes().len(), 2);
            
            // Check route details
            let routes = controller.routes();
            assert_eq!(routes[0].path, "");
            assert_eq!(routes[0].handler_name, "index");
            assert_eq!(routes[1].path, "/info");
            assert_eq!(routes[1].handler_name, "info");
        }
        Err(e) => {
            panic!("Controller registration failed: {}", e);
        }
    }
}

#[test]
fn test_registry_functionality() {
    // Note: We can't clear the global registry in a real test environment
    // This test demonstrates the functionality without relying on a clean state
    
    // Use a unique name for this test
    let test_controller_name = "AutoRegTestController_Sync";
    
    // Check if already registered to avoid duplicate registration
    if !elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(test_controller_name) {
        // Manually register the controller
        elif_http::bootstrap::register_controller_type(
            test_controller_name,
            || Box::new(AutoRegTestController::new())
        );
    }
    
    // Check if the controller type is in the registry
    let registered_types = elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.get_registered_types();
    assert!(
        registered_types.contains(&test_controller_name.to_string()),
        "{} should be registered. Registered types: {:?}",
        test_controller_name,
        registered_types
    );
    
    // Verify count (at least 1, since there may be other controllers registered)
    assert!(elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.count() >= 1);
    
    // Verify is_registered
    assert!(elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered(test_controller_name));
    assert!(!elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered("NonExistentController"));
}
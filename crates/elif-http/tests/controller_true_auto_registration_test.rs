//! Test for TRUE controller auto-registration (no manual registration needed)
//!
//! This test verifies that the #[controller] macro actually auto-registers
//! controllers without any manual intervention using the ctor-based approach.

#[cfg(feature = "derive")]
mod auto_registration_tests {
    use elif_http::{ElifRequest, ElifResponse, HttpResult};
    use elif_http::controller::{ElifController, ControllerRoute};
    use elif_http::routing::HttpMethod;
    use elif_http_derive::{controller, get};

    #[derive(Debug)]
    pub struct TrueAutoTestController;

    impl TrueAutoTestController {
        pub fn new() -> Self {
            Self
        }
    }

    // This should automatically register the controller via the #[controller] macro
    #[controller("/true-auto")]
    impl TrueAutoTestController {
        #[get("")]
        pub async fn index(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().text("True auto-registration works!"))
        }
        
        #[get("/test")]
        pub async fn test(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().text("Auto test endpoint"))
        }
    }

    #[test]
    fn test_controller_truly_auto_registered() {
        // NO manual registration here!
        // The controller should be automatically registered by the macro
        
        // Check if the controller is in the registry
        let registered_types = elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.get_registered_types();
        
        // Print for debugging
        println!("All registered controller types: {:?}", registered_types);
        
        // Verify the controller was auto-registered
        assert!(
            registered_types.contains(&"TrueAutoTestController".to_string()),
            "TrueAutoTestController should be auto-registered by the macro. Found: {:?}",
            registered_types
        );
        
        // Verify it can be created
        let result = elif_http::bootstrap::create_controller("TrueAutoTestController");
        assert!(result.is_ok(), "Should be able to create auto-registered controller");
        
        let controller = result.unwrap();
        assert_eq!(controller.name(), "TrueAutoTestController");
        assert_eq!(controller.base_path(), "/true-auto");
        assert_eq!(controller.routes().len(), 2);
    }

    #[test]
    fn test_registry_has_auto_registered_controller() {
        // Verify the controller is registered in the global registry
        assert!(
            elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.is_registered("TrueAutoTestController"),
            "Controller should be auto-registered"
        );
        
        // Verify count includes our auto-registered controller
        let count = elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.count();
        assert!(count >= 1, "Registry should contain at least our auto-registered controller");
    }
}

// Test that works even without the derive feature
#[cfg(not(feature = "derive"))]
mod fallback_tests {
    #[test]
    fn test_derive_feature_disabled() {
        // When derive feature is disabled, we can't test auto-registration
        // This test just ensures the code compiles without the derive feature
        let registry_count = elif_http::bootstrap::CONTROLLER_TYPE_REGISTRY.count();
        println!("Registry has {} controllers (derive feature disabled)", registry_count);
    }
}
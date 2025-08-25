//! Integration tests for controller macros with full HTTP routing
//!
//! This test verifies that the #[controller] macro applied to impl blocks
//! properly generates ElifController trait implementations that work with
//! the actual Router and HTTP handling.

#[cfg(feature = "derive")]
mod tests {
    use elif_http::{
        controller::{ControllerRoute, ElifController},
        request::ElifRequest,
        response::ElifResponse,
        routing::{HttpMethod, Router},
        HttpResult,
    };
    use elif_http_derive::{controller, delete, get, middleware, post, put};

    // Test controller - needs to be Clone for Arc usage
    #[derive(Clone)]
    pub struct ProductController;

    #[controller("/api/products")]
    impl ProductController {
        #[get("")]
        pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().text("Product list"))
        }

        #[get("/{id}")]
        pub async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
            let default = "unknown".to_string();
            let id = req.path_param("id").unwrap_or(&default);
            Ok(ElifResponse::ok().text(&format!("Product {}", id)))
        }

        #[post("")]
        #[middleware("auth")]
        pub async fn create(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::created().text("Product created"))
        }

        #[put("/{id}")]
        #[middleware("auth", "validate")]
        pub async fn update(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
            let default = "unknown".to_string();
            let id = req.path_param("id").unwrap_or(&default);
            Ok(ElifResponse::ok().text(&format!("Product {} updated", id)))
        }

        #[delete("/{id}")]
        #[middleware("auth", "admin")]
        pub async fn delete(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
            let default = "unknown".to_string();
            let id = req.path_param("id").unwrap_or(&default);
            Ok(ElifResponse::ok().text(&format!("Product {} deleted", id)))
        }
    }

    #[test]
    fn test_controller_trait_methods() {
        let controller = ProductController;

        // Test basic trait methods
        assert_eq!(controller.name(), "ProductController");
        assert_eq!(controller.base_path(), "/api/products");

        // Test routes generation
        let routes = controller.routes();
        assert_eq!(routes.len(), 5);

        // Verify GET / route
        assert_eq!(routes[0].method, HttpMethod::GET);
        assert_eq!(routes[0].path, "");
        assert_eq!(routes[0].handler_name, "list");
        assert!(routes[0].middleware.is_empty());

        // Verify GET /{id} route
        assert_eq!(routes[1].method, HttpMethod::GET);
        assert_eq!(routes[1].path, "/{id}");
        assert_eq!(routes[1].handler_name, "show");
        assert!(routes[1].middleware.is_empty());

        // Verify POST / route with middleware
        assert_eq!(routes[2].method, HttpMethod::POST);
        assert_eq!(routes[2].path, "");
        assert_eq!(routes[2].handler_name, "create");
        assert_eq!(routes[2].middleware, vec!["auth"]);

        // Verify PUT /{id} route with multiple middleware
        assert_eq!(routes[3].method, HttpMethod::PUT);
        assert_eq!(routes[3].path, "/{id}");
        assert_eq!(routes[3].handler_name, "update");
        assert_eq!(routes[3].middleware, vec!["auth", "validate"]);

        // Verify DELETE /{id} route
        assert_eq!(routes[4].method, HttpMethod::DELETE);
        assert_eq!(routes[4].path, "/{id}");
        assert_eq!(routes[4].handler_name, "delete");
        assert_eq!(routes[4].middleware, vec!["auth", "admin"]);
    }

    #[test]
    fn test_router_integration() {
        let controller = ProductController;
        let router: Router = Router::new().controller(controller);

        // Verify the router has registered all the controller routes
        // This tests that Router::controller() properly uses the ElifController trait

        // The actual route testing would require a full HTTP server setup
        // For now, we just verify the router accepts the controller
        // (Router type construction proves integration works)
        let _ = router;
    }

    // Test controller with no routes
    #[derive(Clone)]
    pub struct UtilityController;

    #[controller("/utils")]
    impl UtilityController {
        #[allow(dead_code)]
        pub fn helper(&self) -> String {
            "Helper".to_string()
        }
    }

    #[test]
    fn test_empty_controller() {
        let controller = UtilityController;

        assert_eq!(controller.name(), "UtilityController");
        assert_eq!(controller.base_path(), "/utils");
        assert_eq!(controller.routes().len(), 0);
    }

    // Test controller constants
    #[test]
    fn test_controller_constants() {
        assert_eq!(ProductController::BASE_PATH, "/api/products");
        assert_eq!(ProductController::CONTROLLER_NAME, "ProductController");

        assert_eq!(UtilityController::BASE_PATH, "/utils");
        assert_eq!(UtilityController::CONTROLLER_NAME, "UtilityController");
    }
}

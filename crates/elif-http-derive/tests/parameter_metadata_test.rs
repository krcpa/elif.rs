use elif_http::controller::{ControllerRoute, ElifController, RouteParam};
use elif_http::routing::{params::ParamType, HttpMethod};
use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http_derive::{controller, get, post};

// Test controller with various parameter scenarios
#[derive(Default)]
pub struct TestController;

impl TestController {
    pub fn new() -> Self {
        Self
    }
}

#[controller("/api/test")]
impl TestController {
    // Test 1: Basic parameter extraction without type annotation
    #[get("/users/{id}")]
    async fn get_user(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("User"))
    }

    // Test 2: Parameter with default type (String)
    #[get("/posts/{post_id}")]
    async fn get_post(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("Post"))
    }

    // Test 3: Multiple parameters without type annotations
    #[get("/users/{user_id}/posts/{post_id}")]
    async fn get_user_post(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("User Post"))
    }

    // Test 4: No parameters
    #[get("/health")]
    async fn health(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("OK"))
    }

    // Test 5: Another parameter test
    #[post("/search/{query}")]
    async fn search(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("Search"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_extraction() {
        let controller = TestController;
        let routes = controller.routes();

        // Test 1: Basic parameter extraction (default to String)
        let get_user_route = routes
            .iter()
            .find(|r| r.handler_name == "get_user")
            .unwrap();
        assert_eq!(get_user_route.params.len(), 1);
        assert_eq!(get_user_route.params[0].name, "id");
        assert_eq!(get_user_route.params[0].param_type, ParamType::String);

        // Test 2: Parameter with default type (String)
        let get_post_route = routes
            .iter()
            .find(|r| r.handler_name == "get_post")
            .unwrap();
        assert_eq!(get_post_route.params.len(), 1);
        assert_eq!(get_post_route.params[0].name, "post_id");
        assert_eq!(get_post_route.params[0].param_type, ParamType::String);

        // Test 3: Multiple parameters (both default to String)
        let get_user_post_route = routes
            .iter()
            .find(|r| r.handler_name == "get_user_post")
            .unwrap();
        assert_eq!(get_user_post_route.params.len(), 2);

        let user_id_param = get_user_post_route
            .params
            .iter()
            .find(|p| p.name == "user_id")
            .unwrap();
        assert_eq!(user_id_param.param_type, ParamType::String);

        let post_id_param = get_user_post_route
            .params
            .iter()
            .find(|p| p.name == "post_id")
            .unwrap();
        assert_eq!(post_id_param.param_type, ParamType::String);

        // Test 4: No parameters
        let health_route = routes.iter().find(|r| r.handler_name == "health").unwrap();
        assert_eq!(health_route.params.len(), 0);

        // Test 5: Another parameter with default type
        let search_route = routes.iter().find(|r| r.handler_name == "search").unwrap();
        assert_eq!(search_route.params.len(), 1);
        assert_eq!(search_route.params[0].name, "query");
        assert_eq!(search_route.params[0].param_type, ParamType::String);
    }

    #[test]
    fn test_route_metadata_completeness() {
        let controller = TestController;
        let routes = controller.routes();

        // Verify we have all expected routes
        assert_eq!(routes.len(), 5);

        // Verify each route has the expected structure
        for route in &routes {
            assert!(!route.handler_name.is_empty());
            assert!(!route.path.is_empty());
            assert!(matches!(route.method, HttpMethod::GET | HttpMethod::POST));
            // params field should be properly populated (tested above)
        }
    }

    #[test]
    fn test_controller_introspection() {
        let controller = TestController;

        assert_eq!(controller.name(), "TestController");
        assert_eq!(controller.base_path(), "/api/test");

        let routes = controller.routes();
        assert!(!routes.is_empty());
    }
}

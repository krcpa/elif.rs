use elif_http::controller::{ControllerRoute, ElifController, RouteParam};
use elif_http::routing::{params::ParamType, HttpMethod};
use elif_http::{ElifRequest, ElifResponse, HttpResult};
use elif_http_derive::{controller, get};

// Test controller just for metadata extraction
#[derive(Default)]
pub struct TypedController;

impl TypedController {
    pub fn new() -> Self {
        Self
    }
}

#[controller("/api/typed")]
impl TypedController {
    // Test basic route with parameter
    #[get("/test/{id}")]
    async fn test_route(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().text("Test"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parameter_extraction() {
        let controller = TypedController;
        let routes = controller.routes();

        // Test basic parameter extraction
        let test_route = routes
            .iter()
            .find(|r| r.handler_name == "test_route")
            .unwrap();
        assert_eq!(test_route.params.len(), 1);
        assert_eq!(test_route.params[0].name, "id");
        assert_eq!(test_route.params[0].param_type, ParamType::String); // Default type
    }

    #[test]
    fn test_controller_metadata() {
        let controller = TypedController;

        assert_eq!(controller.name(), "TypedController");
        assert_eq!(controller.base_path(), "/api/typed");

        let routes = controller.routes();
        assert_eq!(routes.len(), 1);
    }
}

use axum::Router;

pub fn router() -> Router {
    Router::new()
        // Routes will be added here by `elif route add` command
        // Example: .route("/hello", get(crate::controllers::hello_controller))
}

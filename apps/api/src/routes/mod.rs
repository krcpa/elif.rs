mod todo;

use axum::Router;

pub fn router() -> Router {
    Router::new()
        .nest("/todos", todo::router())
}
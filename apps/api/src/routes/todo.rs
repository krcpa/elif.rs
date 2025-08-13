use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, patch, delete},
    Router,
};
use serde_json::Value;
use uuid::Uuid;

pub fn router() -> Router {
    Router::new()
        .route("/", post(create_Todo))
        .route("/", get(list_Todo))
        .route("/:id", get(get_Todo))
        .route("/:id", patch(update_Todo))
        .route("/:id", delete(delete_Todo))
}

// <<<ELIF:BEGIN agent-editable:create_Todo>>>
async fn create_Todo(
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement create logic
    Ok(Json(serde_json::json!({"id": "placeholder"})))
}
// <<<ELIF:END agent-editable:create_Todo>>>

// <<<ELIF:BEGIN agent-editable:list_Todo>>>
async fn list_Todo() -> Result<Json<Value>, StatusCode> {
    // TODO: Implement list logic
    Ok(Json(serde_json::json!({"items": [], "next": null})))
}
// <<<ELIF:END agent-editable:list_Todo>>>

// <<<ELIF:BEGIN agent-editable:get_Todo>>>
async fn get_Todo(
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement get logic
    Ok(Json(serde_json::json!({"id": id})))
}
// <<<ELIF:END agent-editable:get_Todo>>>

// <<<ELIF:BEGIN agent-editable:update_Todo>>>
async fn update_Todo(
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement update logic
    Ok(Json(serde_json::json!({"id": id})))
}
// <<<ELIF:END agent-editable:update_Todo>>>

// <<<ELIF:BEGIN agent-editable:delete_Todo>>>
async fn delete_Todo(
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement delete logic
    Ok(StatusCode::NO_CONTENT)
}
// <<<ELIF:END agent-editable:delete_Todo>>>

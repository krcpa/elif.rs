use elif_core::ElifError;
use std::collections::HashMap;

pub fn render_template(template: &str, context: &HashMap<&str, String>) -> Result<String, ElifError> {
    let mut result = template.to_string();
    
    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    Ok(result)
}

pub static MODEL_TEMPLATE: &str = r#"use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {{name}} {
    {{fields}}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Create{{name}} {
    {{fields}}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Update{{name}} {
    {{fields}}
}
"#;

pub static HANDLER_TEMPLATE: &str = r#"use axum::{
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
        .route("/", post(create_{{name}}))
        .route("/", get(list_{{name}}))
        .route("/:id", get(get_{{name}}))
        .route("/:id", patch(update_{{name}}))
        .route("/:id", delete(delete_{{name}}))
}

// <<<ELIF:BEGIN agent-editable:create_{{name}}>>>
async fn create_{{name}}(
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement create logic
    Ok(Json(serde_json::json!({"id": "placeholder"})))
}
// <<<ELIF:END agent-editable:create_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:list_{{name}}>>>
async fn list_{{name}}() -> Result<Json<Value>, StatusCode> {
    // TODO: Implement list logic
    Ok(Json(serde_json::json!({"items": [], "next": null})))
}
// <<<ELIF:END agent-editable:list_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:get_{{name}}>>>
async fn get_{{name}}(
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement get logic
    Ok(Json(serde_json::json!({"id": id})))
}
// <<<ELIF:END agent-editable:get_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:update_{{name}}>>>
async fn update_{{name}}(
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement update logic
    Ok(Json(serde_json::json!({"id": id})))
}
// <<<ELIF:END agent-editable:update_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:delete_{{name}}>>>
async fn delete_{{name}}(
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement delete logic
    Ok(StatusCode::NO_CONTENT)
}
// <<<ELIF:END agent-editable:delete_{{name}}>>>
"#;

pub static MIGRATION_TEMPLATE: &str = r#"CREATE TABLE {{table}} (
{{fields}}
);

{{indexes}}
"#;

pub static TEST_TEMPLATE: &str = r#"use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_{{name}}_crud() {
    let app = /* TODO: Create test app */;
    let server = TestServer::new(app).unwrap();
    
    // Test create
    let response = server
        .post("{{route}}")
        .json(&json!({"title": "Test item"}))
        .await;
    
    assert_eq!(response.status_code(), 201);
    
    // Test list
    let response = server.get("{{route}}").await;
    assert_eq!(response.status_code(), 200);
}
"#;
use elif_core::ElifError;
use std::collections::HashMap;

pub fn render_template(
    template: &str,
    context: &HashMap<&str, String>,
) -> Result<String, ElifError> {
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

pub static HANDLER_TEMPLATE: &str = r#"use elif_http::{
    ElifRouter, ElifResponse, ElifJson, ElifPath,
    HttpResult, HttpError, StatusCode,
};
use serde_json::Value;
use uuid::Uuid;

pub fn router() -> ElifRouter {
    ElifRouter::new()
        .post("/", create_{{name}})
        .get("/", list_{{name}})
        .get("/:id", get_{{name}})
        .patch("/:id", update_{{name}})
        .delete("/:id", delete_{{name}})
}

// <<<ELIF:BEGIN agent-editable:create_{{name}}>>>
async fn create_{{name}}(
    payload: Value,
) -> HttpResult<ElifResponse> {
    // TODO: Implement create logic
    Ok(ElifResponse::json(serde_json::json!({"id": "placeholder"}))
        .with_status(StatusCode::CREATED))
}
// <<<ELIF:END agent-editable:create_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:list_{{name}}>>>
async fn list_{{name}}() -> HttpResult<ElifResponse> {
    // TODO: Implement list logic
    Ok(ElifResponse::json(serde_json::json!({"items": [], "next": null}))
        .with_status(StatusCode::OK))
}
// <<<ELIF:END agent-editable:list_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:get_{{name}}>>>
async fn get_{{name}}(
    id: Uuid,
) -> HttpResult<ElifResponse> {
    // TODO: Implement get logic
    Ok(ElifResponse::json(serde_json::json!({"id": id}))
        .with_status(StatusCode::OK))
}
// <<<ELIF:END agent-editable:get_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:update_{{name}}>>>
async fn update_{{name}}(
    id: Uuid,
    payload: Value,
) -> HttpResult<ElifResponse> {
    // TODO: Implement update logic
    Ok(ElifResponse::json(serde_json::json!({"id": id}))
        .with_status(StatusCode::OK))
}
// <<<ELIF:END agent-editable:update_{{name}}>>>

// <<<ELIF:BEGIN agent-editable:delete_{{name}}>>>
async fn delete_{{name}}(
    id: Uuid,
) -> HttpResult<ElifResponse> {
    // TODO: Implement delete logic
    Ok(ElifResponse::empty().with_status(StatusCode::NO_CONTENT))
}
// <<<ELIF:END agent-editable:delete_{{name}}>>>
"#;

pub static MIGRATION_TEMPLATE: &str = r#"CREATE TABLE {{table}} (
{{fields}}
);

{{indexes}}
"#;

pub static TEST_TEMPLATE: &str = r#"use elif_http::{StatusCode, ElifResponse};
use elif_core::{Container, container::test_implementations::*};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_{{name}}_crud() {
    // Setup test container with DI services
    let config = Arc::new(create_test_config());
    let database = Arc::new(TestDatabase::new()) as Arc<dyn elif_core::DatabaseConnection>;
    
    let _container = Container::builder()
        .config(config)
        .database(database)
        .build()
        .unwrap();
    
    // TODO: Create test server using framework abstractions
    // let server = TestServer::new(create_app()).unwrap();
    
    // Test create
    // let response = server
    //     .post("{{route}}")
    //     .json(&json!({"title": "Test item"}))
    //     .await;
    // 
    // assert_eq!(response.status_code(), StatusCode::CREATED);
    
    // Test list
    // let response = server.get("{{route}}").await;
    // assert_eq!(response.status_code(), StatusCode::OK);
    
    // For now, basic assertion to verify test compiles
    assert!(true, "Framework-native test template");
}
"#;

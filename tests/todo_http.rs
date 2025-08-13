use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_Todo_crud() {
    let app = /* TODO: Create test app */;
    let server = TestServer::new(app).unwrap();
    
    // Test create
    let response = server
        .post("/todos")
        .json(&json!({"title": "Test item"}))
        .await;
    
    assert_eq!(response.status_code(), 201);
    
    // Test list
    let response = server.get("/todos").await;
    assert_eq!(response.status_code(), 200);
}

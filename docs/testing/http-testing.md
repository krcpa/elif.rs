# HTTP Testing

Write readable HTTP tests with the fluent test client and assertions from `elif-testing`.

Basic flow
```rust
use elif_testing::{TestClient, TestAssertions};

#[tokio::test]
async fn creates_post() -> anyhow::Result<()> {
    let client = TestClient::with_base_url("http://127.0.0.1:3000");
    let resp = client
        .post("/api/v1/posts")
        .json(&serde_json::json!({"title": "Hello", "content": "..."}))
        .send()
        .await?;

    resp.assert_status(201)
        .assert_json_contains(vec![("title", "Hello")])?;
    Ok(())
}
```

Headers and auth
```rust
let resp = client
    .get("/api/v1/posts")
    .header("authorization", "Bearer TEST")
    .send().await?;
resp.assert_status(200);
```

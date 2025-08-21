# Database Testing

Use transactions/factories for fast, isolated tests and assert DB state.

Factories and seeding
```rust
use elif_testing::database::DatabaseAssertions;
use serde_json::json;

#[tokio::test]
async fn seeds_and_asserts() -> anyhow::Result<()> {
    let db = /* create Pool<Postgres> */;
    let asrt = DatabaseAssertions::new(&db);

    // Seed
    asrt.seed_from_json(json!({
        "users": [
            {"id": 1, "email": "a@example.com"},
            {"id": 2, "email": "b@example.com"}
        ]
    })).await?;

    // Assertions
    asrt.assert_record_exists("users", &[("email", &"a@example.com")]).await?;
    asrt.assert_record_count("users", 2, &[]).await?;
    Ok(())
}
```

Transactional tests
- Wrap each test in a DB transaction and roll it back to isolate state.
- If using a test harness, register a fixture that begins/rolls back automatically.

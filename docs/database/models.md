# Models

Models implement the `Model` trait and provide table metadata, primary key accessors, and (via derive in your app) mapping to/from rows.

Core trait
```rust
use elif_orm::model::Model;

pub trait Model {
    type PrimaryKey: Clone + Send + Sync + std::fmt::Display + Default;
    fn table_name() -> &'static str;
    fn primary_key(&self) -> Option<Self::PrimaryKey>;
    fn set_primary_key(&mut self, key: Self::PrimaryKey);
    // ... timestamps/soft-deletes helpers ...
}
```

Example (app-level)
```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl elif_orm::model::Model for Post {
    type PrimaryKey = i64;
    fn table_name() -> &'static str { "posts" }
    fn primary_key(&self) -> Option<Self::PrimaryKey> { Some(self.id) }
    fn set_primary_key(&mut self, key: Self::PrimaryKey) { self.id = key; }
}
```

CRUD via QueryBuilder
```rust
use elif_orm::query::QueryBuilder;
use sqlx::{Pool, Postgres};

async fn list(pool: &Pool<Postgres>) -> elif_orm::error::ModelResult<Vec<Post>> {
    QueryBuilder::<Post>::new()
        .select(["id", "title"]) // optional
        .from("posts")
        .order_by("id", true)
        .get(pool)
        .await
}
```

# Query Builder

Compose queries fluently with `QueryBuilder<T>`. Fetch models, count rows, and paginate. Execution uses `sqlx` under the hood.

Basic select
```rust
use elif_orm::query::QueryBuilder;
use sqlx::{Pool, Postgres};

async fn titles(pool: &Pool<Postgres>) -> anyhow::Result<Vec<Post>> {
    let posts = QueryBuilder::<Post>::new()
        .select(["id", "title"]) // optional
        .from("posts")
        .order_by("id", true)
        .get(pool)
        .await?;
    Ok(posts)
}
```

First/Count
```rust
let first = QueryBuilder::<Post>::new().from("posts").first(pool).await?;
let count = QueryBuilder::<Post>::new().from("posts").count(pool).await?;
```

Pagination
```rust
use elif_orm::query::QueryBuilder;
let page = 1; let per_page = 20;
let posts = QueryBuilder::<Post>::new()
    .from("posts")
    .paginate(per_page, page)
    .get(pool)
    .await?;
```

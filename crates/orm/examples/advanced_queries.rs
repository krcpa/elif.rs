//! Example: Advanced ORM queries with the QueryBuilder
//!
//! This example demonstrates complex database queries using the elif-orm
//! QueryBuilder with joins, subqueries, aggregations, and more.
use chrono::{DateTime, Utc};
#[allow(unused_imports)]
use elif_orm::{Model, ModelResult, QueryBuilder};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

// Example model for User
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub age: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Implement Model trait for User
impl Model for User {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "users"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = key;
    }

    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        Ok(User {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            name: row.try_get("name")?,
            age: row.try_get("age")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        // This would be implemented with actual field serialization
        HashMap::new()
    }
}

// Example model for Post
#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Model for Post {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "posts"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = key;
    }

    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        Ok(Post {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            content: row.try_get("content")?,
            published: row.try_get("published")?,
            view_count: row.try_get("view_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Main function demonstrating query building patterns
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced ORM Queries Demo");
    println!("============================");

    println!("📝 This demo shows QueryBuilder API patterns:");

    println!("🔍 Basic Queries:");
    println!("   User::query().where_eq('active', true).get(pool)");
    println!("   Post::query().where_like('title', '%rust%').order_by('created_at').get(pool)");

    println!("🔗 Join Queries:");
    println!("   User::query().join('posts', 'users.id', 'posts.user_id').get(pool)");

    println!("📊 Aggregations:");
    println!("   Post::query().select('COUNT(*) as total').count(pool)");

    println!("✅ Query API demonstration completed!");
    println!("   Connect to a PostgreSQL database to execute these queries");

    Ok(())
}

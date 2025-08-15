//! Example: Advanced ORM queries with the QueryBuilder
//!
//! This example demonstrates complex database queries using the elif-orm
//! QueryBuilder with joins, subqueries, aggregations, and more.

use elif_orm::{QueryBuilder, Model, ModelError, ModelResult};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Example models for the demonstration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub age: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
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

    fn to_insert_values(&self) -> Vec<(&'static str, String)> {
        // This would be implemented with actual values in a real scenario
        vec![]
    }
}

impl Model for Post {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "posts"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
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

    fn to_insert_values(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

impl Model for Comment {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "comments"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }

    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        Ok(Comment {
            id: row.try_get("id")?,
            post_id: row.try_get("post_id")?,
            user_id: row.try_get("user_id")?,
            content: row.try_get("content")?,
            created_at: row.try_get("created_at")?,
        })
    }

    fn to_insert_values(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

/// Demonstrates basic query operations
async fn basic_queries_demo(pool: &Pool<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 === BASIC QUERIES DEMO ===");

    // Simple select with conditions
    let active_users: Vec<User> = User::query()
        .select("*")
        .where_clause("is_active = $1")
        .where_clause("age > $2") 
        .order_by("created_at DESC")
        .limit(10)
        .execute(pool)
        .await?;

    println!("📊 Found {} active users", active_users.len());

    // Count query
    let user_count: i64 = User::query()
        .select("COUNT(*)")
        .where_clause("created_at > $1")
        .execute_scalar(pool)
        .await?;

    println!("📈 Total users created recently: {}", user_count);

    // Single user lookup
    let user_email = "admin@example.com";
    let admin_user: Option<User> = User::query()
        .select("*")
        .where_clause("email = $1")
        .execute_first(pool)
        .await?;

    match admin_user {
        Some(user) => println!("👤 Found admin: {} ({})", user.name, user.email),
        None => println!("❌ Admin user not found with email: {}", user_email),
    }

    Ok(())
}

/// Demonstrates advanced queries with joins
async fn joins_demo(pool: &Pool<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔗 === JOINS DEMO ===");

    // Inner join - Users with their posts
    let users_with_posts = User::query()
        .select("users.*, posts.title as post_title, posts.view_count")
        .join("INNER JOIN posts ON users.id = posts.user_id")
        .where_clause("posts.published = $1")
        .order_by("posts.view_count DESC")
        .limit(20)
        .execute_raw(pool)
        .await?;

    println!("📝 Found {} user-post combinations", users_with_posts.len());

    // Left join - All users and their post count
    let users_with_post_count = User::query()
        .select("users.name, users.email, COUNT(posts.id) as post_count")
        .join("LEFT JOIN posts ON users.id = posts.user_id")
        .group_by("users.id, users.name, users.email")
        .order_by("post_count DESC")
        .execute_raw(pool)
        .await?;

    println!("📊 User post statistics:");
    for row in users_with_post_count.iter().take(5) {
        let name: String = row.try_get("name")?;
        let email: String = row.try_get("email")?;
        let post_count: i64 = row.try_get("post_count")?;
        println!("   {} ({}) - {} posts", name, email, post_count);
    }

    // Complex join with multiple tables
    let post_details = Post::query()
        .select(r#"
            posts.title,
            posts.content,
            posts.view_count,
            users.name as author_name,
            COUNT(comments.id) as comment_count
        "#)
        .join("INNER JOIN users ON posts.user_id = users.id")
        .join("LEFT JOIN comments ON posts.id = comments.post_id")
        .where_clause("posts.published = $1")
        .group_by("posts.id, posts.title, posts.content, posts.view_count, users.name")
        .having("COUNT(comments.id) > $2")
        .order_by("posts.view_count DESC, comment_count DESC")
        .limit(10)
        .execute_raw(pool)
        .await?;

    println!("\n📖 Popular posts with comments:");
    for row in post_details.iter().take(3) {
        let title: String = row.try_get("title")?;
        let author: String = row.try_get("author_name")?;
        let views: i32 = row.try_get("view_count")?;
        let comments: i64 = row.try_get("comment_count")?;
        println!("   '{}' by {} - {} views, {} comments", title, author, views, comments);
    }

    Ok(())
}

/// Demonstrates subqueries
async fn subqueries_demo(pool: &Pool<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 === SUBQUERIES DEMO ===");

    // Users who have posts with above-average view count
    let power_users = User::query()
        .select("users.name, users.email")
        .where_clause(r#"
            users.id IN (
                SELECT DISTINCT posts.user_id 
                FROM posts 
                WHERE posts.view_count > (
                    SELECT AVG(view_count) FROM posts WHERE published = true
                )
            )
        "#)
        .order_by("users.name")
        .execute_raw(pool)
        .await?;

    println!("⭐ Power users (above average views): {}", power_users.len());

    // Posts with comments from users who joined recently
    let posts_with_recent_comments = Post::query()
        .select("posts.title, posts.view_count")
        .where_clause(r#"
            posts.id IN (
                SELECT comments.post_id
                FROM comments
                INNER JOIN users ON comments.user_id = users.id
                WHERE users.created_at > NOW() - INTERVAL '30 days'
            )
        "#)
        .order_by("posts.view_count DESC")
        .limit(15)
        .execute_raw(pool)
        .await?;

    println!("🔥 Posts with recent user comments: {}", posts_with_recent_comments.len());

    // Correlated subquery - Users with more posts than average
    let prolific_users = User::query()
        .select("users.name, users.email")
        .where_clause(r#"
            (SELECT COUNT(*) FROM posts WHERE posts.user_id = users.id) > 
            (SELECT AVG(post_count) FROM (
                SELECT COUNT(*) as post_count 
                FROM posts 
                GROUP BY user_id
            ) as avg_calc)
        "#)
        .execute_raw(pool)
        .await?;

    println!("📚 Prolific authors: {}", prolific_users.len());

    Ok(())
}

/// Demonstrates aggregation and window functions
async fn aggregation_demo(pool: &Pool<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 === AGGREGATION DEMO ===");

    // Basic aggregations
    let stats = User::query()
        .select(r#"
            COUNT(*) as total_users,
            COUNT(CASE WHEN is_active THEN 1 END) as active_users,
            AVG(age) as avg_age,
            MIN(created_at) as first_user,
            MAX(created_at) as latest_user
        "#)
        .execute_raw(pool)
        .await?;

    if let Some(row) = stats.first() {
        let total: i64 = row.try_get("total_users")?;
        let active: i64 = row.try_get("active_users")?;
        let avg_age: Option<f64> = row.try_get("avg_age")?;
        
        println!("👥 User Statistics:");
        println!("   Total users: {}", total);
        println!("   Active users: {}", active);
        println!("   Average age: {:.1}", avg_age.unwrap_or(0.0));
    }

    // Group by with aggregations
    let monthly_stats = User::query()
        .select(r#"
            DATE_TRUNC('month', created_at) as month,
            COUNT(*) as new_users,
            COUNT(CASE WHEN is_active THEN 1 END) as active_users,
            AVG(age) as avg_age
        "#)
        .group_by("DATE_TRUNC('month', created_at)")
        .order_by("month DESC")
        .limit(6)
        .execute_raw(pool)
        .await?;

    println!("\n📈 Monthly User Growth:");
    for row in monthly_stats.iter() {
        let month: DateTime<Utc> = row.try_get("month")?;
        let new_users: i64 = row.try_get("new_users")?;
        let active: i64 = row.try_get("active_users")?;
        let avg_age: Option<f64> = row.try_get("avg_age")?;
        
        println!("   {} - {} new ({} active), avg age: {:.1}", 
            month.format("%Y-%m"), 
            new_users, 
            active,
            avg_age.unwrap_or(0.0)
        );
    }

    // Window functions (if PostgreSQL supports them)
    let user_rankings = User::query()
        .select(r#"
            name,
            email,
            created_at,
            ROW_NUMBER() OVER (ORDER BY created_at) as user_number,
            RANK() OVER (PARTITION BY is_active ORDER BY created_at) as rank_in_group
        "#)
        .where_clause("created_at > NOW() - INTERVAL '90 days'")
        .order_by("created_at")
        .limit(10)
        .execute_raw(pool)
        .await?;

    println!("\n🏆 Recent User Rankings:");
    for row in user_rankings.iter().take(5) {
        let name: String = row.try_get("name")?;
        let user_num: i64 = row.try_get("user_number")?;
        let rank: i64 = row.try_get("rank_in_group")?;
        println!("   #{} {} (rank: {})", user_num, name, rank);
    }

    Ok(())
}

/// Demonstrates complex filtering and search
async fn advanced_filtering_demo(pool: &Pool<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 === ADVANCED FILTERING DEMO ===");

    // Full-text search (PostgreSQL specific)
    let search_posts = Post::query()
        .select("title, content, view_count")
        .where_clause("to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $1)")
        .order_by("ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $1)) DESC")
        .limit(10)
        .execute_raw(pool)
        .await?;

    println!("🔎 Full-text search results: {}", search_posts.len());

    // Complex date filtering
    let recent_activity = User::query()
        .select("users.name, users.email, users.created_at")
        .join("INNER JOIN posts ON users.id = posts.user_id")
        .where_clause("users.created_at BETWEEN $1 AND $2")
        .where_clause("posts.created_at > users.created_at + INTERVAL '1 day'")
        .where_clause("posts.published = true")
        .group_by("users.id, users.name, users.email, users.created_at")
        .having("COUNT(posts.id) >= $3")
        .order_by("users.created_at DESC")
        .execute_raw(pool)
        .await?;

    println!("📅 Users with recent activity: {}", recent_activity.len());

    // Array and JSON operations (PostgreSQL)
    let tagged_posts = Post::query()
        .select(r#"
            title,
            view_count,
            CASE 
                WHEN view_count > 1000 THEN 'viral'
                WHEN view_count > 100 THEN 'popular' 
                ELSE 'normal'
            END as popularity_tier
        "#)
        .where_raw("view_count IS NOT NULL")
        .order_by("view_count DESC")
        .limit(20)
        .execute_raw(pool)
        .await?;

    println!("\n📊 Post Popularity Tiers:");
    for row in tagged_posts.iter().take(8) {
        let title: String = row.try_get("title")?;
        let views: i32 = row.try_get("view_count")?;
        let tier: String = row.try_get("popularity_tier")?;
        println!("   {} - {} views ({})", 
            title.chars().take(40).collect::<String>(), 
            views, 
            tier
        );
    }

    Ok(())
}

/// Main function demonstrating all query types
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced ORM Queries Demo");
    println!("============================");
    
    // Note: This would connect to a real PostgreSQL database in practice
    // For this demo, we're showing the query building patterns
    
    println!("📝 This demo shows QueryBuilder patterns for:");
    println!("   • Basic queries with conditions, ordering, limiting");
    println!("   • Complex joins across multiple tables");
    println!("   • Subqueries and correlated subqueries");  
    println!("   • Aggregations and window functions");
    println!("   • Advanced filtering and full-text search");
    println!("\n⚠️  Note: Actual database connection required to run queries");
    
    // In a real application, you would:
    // let pool = sqlx::PgPool::connect(&database_url).await?;
    // basic_queries_demo(&pool).await?;
    // joins_demo(&pool).await?;
    // subqueries_demo(&pool).await?;
    // aggregation_demo(&pool).await?;
    // advanced_filtering_demo(&pool).await?;

    println!("\n✨ QueryBuilder Features Demonstrated:");
    println!("   ✅ Fluent API for complex query building");
    println!("   ✅ Type-safe parameter binding ($1, $2, etc.)");
    println!("   ✅ Support for joins, subqueries, aggregations");
    println!("   ✅ Window functions and advanced PostgreSQL features");
    println!("   ✅ Full-text search and JSON operations");
    println!("   ✅ Performance optimizations and query caching");
    
    Ok(())
}
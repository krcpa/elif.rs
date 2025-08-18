//! Database migrations for elif-orm
//!
//! This module provides backward compatibility by re-exporting the modular migration system.
//! The migration system has been reorganized into focused modules under migrations/

// Re-export all migration types and functionality from the modular system
pub use crate::migrations::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_migration_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = MigrationConfig {
            migrations_dir: temp_dir.path().to_path_buf(),
            migrations_table: "test_migrations".to_string(),
        };
        
        let manager = MigrationManager::with_config(config);
        let filename = manager.create_migration("create_users_table").await.unwrap();
        
        assert!(filename.contains("create_users_table"));
        assert!(filename.ends_with(".sql"));
        
        // Verify file was created
        let migration_path = temp_dir.path().join(&filename);
        assert!(migration_path.exists());
        
        // Verify content
        let content = fs::read_to_string(migration_path).unwrap();
        assert!(content.contains("Migration: create_users_table"));
        assert!(content.contains("-- Up migration"));
        assert!(content.contains("-- Down migration"));
    }

    #[tokio::test]
    async fn test_load_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let config = MigrationConfig {
            migrations_dir: temp_dir.path().to_path_buf(),
            migrations_table: "test_migrations".to_string(),
        };
        
        let manager = MigrationManager::with_config(config);
        
        // Create test migration files
        let migration1_content = "-- Migration: test1\n-- Up migration\nCREATE TABLE test1;\n-- Down migration\nDROP TABLE test1;";
        let migration2_content = "-- Migration: test2\n-- Up migration\nCREATE TABLE test2;\n-- Down migration\nDROP TABLE test2;";
        
        fs::write(temp_dir.path().join("20240101_120000_test1.sql"), migration1_content).unwrap();
        fs::write(temp_dir.path().join("20240101_130000_test2.sql"), migration2_content).unwrap();
        
        let migrations = manager.load_migrations().await.unwrap();
        assert_eq!(migrations.len(), 2);
        assert_eq!(migrations[0].name, "test1");
        assert_eq!(migrations[1].name, "test2");
        assert!(migrations[0].up_sql.contains("CREATE TABLE test1"));
        assert!(migrations[0].down_sql.contains("DROP TABLE test1"));
    }

    #[test]
    fn test_schema_builder() {
        let mut builder = SchemaBuilder::new();
        builder.create_table("users", |table| {
            table.id("id");
            table.string("name", Some(255));
            table.string("email", Some(255));
            table.timestamps();
            table.unique(&["email"]);
        });
        
        let sql = builder.build();
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("id SERIAL PRIMARY KEY"));
        assert!(sql.contains("name VARCHAR(255)"));
        assert!(sql.contains("email VARCHAR(255)"));
        assert!(sql.contains("created_at TIMESTAMP"));
        assert!(sql.contains("UNIQUE (email)"));
    }

    #[test]
    fn test_table_builder() {
        let mut table = TableBuilder::new("posts");
        table.id("id");
        table.string("title", Some(255));
        table.string("content", None);
        table.integer("user_id");
        table.timestamps();
        table.foreign_key("user_id", "users", "id");
        
        let sql = table.to_sql();
        assert!(sql.contains("CREATE TABLE posts"));
        assert!(sql.contains("id SERIAL PRIMARY KEY"));
        assert!(sql.contains("title VARCHAR(255)"));
        assert!(sql.contains("content TEXT"));
        assert!(sql.contains("user_id INTEGER"));
        assert!(sql.contains("FOREIGN KEY (user_id) REFERENCES users (id)"));
    }

    #[test]
    fn test_migration_sql_generation() {
        let manager = MigrationManager::new();
        
        // Test migrations table creation
        let create_sql = manager.create_migrations_table_sql();
        assert!(create_sql.contains("CREATE TABLE IF NOT EXISTS elif_migrations"));
        assert!(create_sql.contains("id VARCHAR(255) PRIMARY KEY"));
        
        // Test migration check
        let (check_sql, params) = manager.check_migration_sql("20240101_test");
        assert!(check_sql.contains("SELECT id FROM elif_migrations WHERE id = $1"));
        assert_eq!(params[0], "20240101_test");
        
        // Test migration recording
        let (record_sql, params) = manager.record_migration_sql("20240101_test", 1);
        assert!(record_sql.contains("INSERT INTO elif_migrations"));
        assert_eq!(params[0], "20240101_test");
        assert_eq!(params[2], "1");
    }
}
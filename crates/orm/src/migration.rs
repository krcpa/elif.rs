//! Database migrations for elif-orm
//!
//! This module provides a comprehensive migration system for database schema changes.
//! It supports creating migrations, tracking applied migrations, and basic schema operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{OrmError, OrmResult};

/// Represents a database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Unique identifier for the migration (typically timestamp)
    pub id: String,
    /// Human-readable name for the migration
    pub name: String,
    /// SQL statements to apply the migration
    pub up_sql: String,
    /// SQL statements to rollback the migration
    pub down_sql: String,
    /// When the migration was created
    pub created_at: DateTime<Utc>,
}

/// Migration status in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration ID
    pub id: String,
    /// When the migration was applied
    pub applied_at: DateTime<Utc>,
    /// Batch number (for grouping migrations)
    pub batch: i32,
}

/// Configuration for the migration system
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Directory where migration files are stored
    pub migrations_dir: PathBuf,
    /// Table name for tracking migrations
    pub migrations_table: String,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            migrations_dir: PathBuf::from("migrations"),
            migrations_table: "elif_migrations".to_string(),
        }
    }
}

/// Migration manager for creating and running migrations
pub struct MigrationManager {
    config: MigrationConfig,
}

impl MigrationManager {
    /// Create a new migration manager with default configuration
    pub fn new() -> Self {
        Self::with_config(MigrationConfig::default())
    }

    /// Create a new migration manager with custom configuration
    pub fn with_config(config: MigrationConfig) -> Self {
        Self { config }
    }

    /// Create a new migration file
    pub async fn create_migration(&self, name: &str) -> OrmResult<String> {
        // Ensure migrations directory exists
        fs::create_dir_all(&self.config.migrations_dir)
            .map_err(|e| OrmError::Migration(format!("Failed to create migrations directory: {}", e)))?;

        // Generate timestamp-based ID
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let migration_id = format!("{}_{}", timestamp, name.replace(' ', "_").to_lowercase());
        let filename = format!("{}.sql", migration_id);
        let filepath = self.config.migrations_dir.join(&filename);

        // Create migration template
        let template = self.create_migration_template(name, &migration_id);
        
        fs::write(&filepath, template)
            .map_err(|e| OrmError::Migration(format!("Failed to write migration file: {}", e)))?;

        Ok(filename)
    }

    /// Load all migration files from the migrations directory
    pub async fn load_migrations(&self) -> OrmResult<Vec<Migration>> {
        if !self.config.migrations_dir.exists() {
            return Ok(Vec::new());
        }

        let mut migrations = Vec::new();
        let entries = fs::read_dir(&self.config.migrations_dir)
            .map_err(|e| OrmError::Migration(format!("Failed to read migrations directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| OrmError::Migration(format!("Failed to read directory entry: {}", e)))?;
            
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "sql") {
                let migration = self.parse_migration_file(&path).await?;
                migrations.push(migration);
            }
        }

        // Sort by migration ID (timestamp)
        migrations.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(migrations)
    }

    /// Parse a migration file into a Migration struct
    async fn parse_migration_file(&self, path: &Path) -> OrmResult<Migration> {
        let content = fs::read_to_string(path)
            .map_err(|e| OrmError::Migration(format!("Failed to read migration file: {}", e)))?;

        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| OrmError::Migration("Invalid migration filename".to_string()))?;

        // Extract ID and name from filename (format: timestamp_name)
        let parts: Vec<&str> = filename.splitn(2, '_').collect();
        if parts.len() < 2 {
            return Err(OrmError::Migration("Migration filename must follow format: timestamp_name".to_string()));
        }

        let id = filename.to_string();
        let name = parts[1..].join("_").replace('_', " ");

        // Parse UP and DOWN sections
        let (up_sql, down_sql) = self.parse_migration_content(&content)?;

        // Try to parse creation date from filename timestamp
        let created_at = self.parse_migration_timestamp(&parts[0])
            .unwrap_or_else(|_| Utc::now());

        Ok(Migration {
            id,
            name,
            up_sql,
            down_sql,
            created_at,
        })
    }

    /// Parse migration content to extract UP and DOWN SQL
    fn parse_migration_content(&self, content: &str) -> OrmResult<(String, String)> {
        let lines: Vec<&str> = content.lines().collect();
        let mut up_sql = Vec::new();
        let mut down_sql = Vec::new();
        let mut current_section = "";

        for line in lines {
            let trimmed = line.trim().to_lowercase();
            
            if trimmed.starts_with("-- up") || trimmed.contains("up migration") {
                current_section = "up";
                continue;
            } else if trimmed.starts_with("-- down") || trimmed.contains("down migration") {
                current_section = "down";
                continue;
            }

            // Skip comment lines and empty lines
            if line.trim().is_empty() || line.trim().starts_with("--") {
                continue;
            }

            match current_section {
                "up" => up_sql.push(line),
                "down" => down_sql.push(line),
                _ => {} // Before any section marker
            }
        }

        Ok((up_sql.join("\n").trim().to_string(), down_sql.join("\n").trim().to_string()))
    }

    /// Parse timestamp from migration filename
    fn parse_migration_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        let formatted = format!("{}000000", &timestamp_str[..8]); // YYYYMMDD -> YYYYMMDD000000
        let naive = chrono::NaiveDateTime::parse_from_str(&formatted, "%Y%m%d%H%M%S")?;
        Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
    }

    /// Create migration template content
    fn create_migration_template(&self, name: &str, migration_id: &str) -> String {
        format!(
            "-- Migration: {}\n\
             -- ID: {}\n\
             -- Created: {}\n\n\
             -- Up migration\n\
             -- Add your schema changes here\n\n\n\
             -- Down migration  \n\
             -- Add rollback statements here\n\n",
            name,
            migration_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// SQL to create the migrations tracking table
    pub fn create_migrations_table_sql(&self) -> String {
        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    \
                id VARCHAR(255) PRIMARY KEY,\n    \
                applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    \
                batch INTEGER NOT NULL\n\
            );",
            self.config.migrations_table
        )
    }

    /// SQL to check if a migration has been applied
    pub fn check_migration_sql(&self, migration_id: &str) -> (String, Vec<String>) {
        (
            format!("SELECT id FROM {} WHERE id = $1", self.config.migrations_table),
            vec![migration_id.to_string()]
        )
    }

    /// SQL to record a migration as applied
    pub fn record_migration_sql(&self, migration_id: &str, batch: i32) -> (String, Vec<String>) {
        (
            format!(
                "INSERT INTO {} (id, applied_at, batch) VALUES ($1, $2, $3)",
                self.config.migrations_table
            ),
            vec![
                migration_id.to_string(),
                Utc::now().to_rfc3339(),
                batch.to_string(),
            ]
        )
    }

    /// SQL to remove a migration record (for rollback)
    pub fn remove_migration_sql(&self, migration_id: &str) -> (String, Vec<String>) {
        (
            format!("DELETE FROM {} WHERE id = $1", self.config.migrations_table),
            vec![migration_id.to_string()]
        )
    }

    /// SQL to get the latest batch number
    pub fn get_latest_batch_sql(&self) -> String {
        format!("SELECT COALESCE(MAX(batch), 0) FROM {}", self.config.migrations_table)
    }

    /// SQL to get applied migrations
    pub fn get_applied_migrations_sql(&self) -> String {
        format!(
            "SELECT id, applied_at, batch FROM {} ORDER BY batch DESC, applied_at DESC",
            self.config.migrations_table
        )
    }
}

/// Basic schema operations for migrations
pub struct SchemaBuilder {
    statements: Vec<String>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a new table
    pub fn create_table<F>(&mut self, table_name: &str, callback: F) -> &mut Self
    where
        F: FnOnce(&mut TableBuilder),
    {
        let mut table_builder = TableBuilder::new(table_name);
        callback(&mut table_builder);
        
        let sql = table_builder.to_sql();
        self.statements.push(sql);
        self
    }

    /// Drop a table
    pub fn drop_table(&mut self, table_name: &str) -> &mut Self {
        self.statements.push(format!("DROP TABLE IF EXISTS {};", table_name));
        self
    }

    /// Add a column to existing table
    pub fn add_column(&mut self, table_name: &str, column_name: &str, column_type: &str) -> &mut Self {
        self.statements.push(format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            table_name, column_name, column_type
        ));
        self
    }

    /// Drop a column from existing table
    pub fn drop_column(&mut self, table_name: &str, column_name: &str) -> &mut Self {
        self.statements.push(format!(
            "ALTER TABLE {} DROP COLUMN {};",
            table_name, column_name
        ));
        self
    }

    /// Create an index
    pub fn create_index(&mut self, table_name: &str, column_names: &[&str], index_name: Option<&str>) -> &mut Self {
        let default_name = format!("idx_{}_{}", table_name, column_names.join("_"));
        let index_name = index_name.unwrap_or(&default_name);
        self.statements.push(format!(
            "CREATE INDEX {} ON {} ({});",
            index_name,
            table_name,
            column_names.join(", ")
        ));
        self
    }

    /// Drop an index
    pub fn drop_index(&mut self, index_name: &str) -> &mut Self {
        self.statements.push(format!("DROP INDEX IF EXISTS {};", index_name));
        self
    }

    /// Get all SQL statements
    pub fn to_sql(&self) -> Vec<String> {
        self.statements.clone()
    }

    /// Execute all statements as a single SQL string
    pub fn build(&self) -> String {
        self.statements.join("\n")
    }
}

/// Table builder for CREATE TABLE statements
pub struct TableBuilder {
    table_name: String,
    columns: Vec<String>,
    constraints: Vec<String>,
}

impl TableBuilder {
    fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
            columns: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Add a column
    pub fn column(&mut self, name: &str, column_type: &str) -> &mut Self {
        self.columns.push(format!("{} {}", name, column_type));
        self
    }

    /// Add an ID column (auto-increment primary key)
    pub fn id(&mut self, name: &str) -> &mut Self {
        self.columns.push(format!("{} SERIAL PRIMARY KEY", name));
        self
    }

    /// Add a UUID column
    pub fn uuid(&mut self, name: &str) -> &mut Self {
        self.columns.push(format!("{} UUID DEFAULT gen_random_uuid()", name));
        self
    }

    /// Add a string column
    pub fn string(&mut self, name: &str, length: Option<u32>) -> &mut Self {
        let column_type = match length {
            Some(len) => format!("VARCHAR({})", len),
            None => "TEXT".to_string(),
        };
        self.columns.push(format!("{} {}", name, column_type));
        self
    }

    /// Add an integer column
    pub fn integer(&mut self, name: &str) -> &mut Self {
        self.columns.push(format!("{} INTEGER", name));
        self
    }

    /// Add a boolean column
    pub fn boolean(&mut self, name: &str) -> &mut Self {
        self.columns.push(format!("{} BOOLEAN", name));
        self
    }

    /// Add timestamp columns
    pub fn timestamps(&mut self) -> &mut Self {
        self.columns.push("created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP".to_string());
        self.columns.push("updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP".to_string());
        self
    }

    /// Add a primary key constraint
    pub fn primary_key(&mut self, columns: &[&str]) -> &mut Self {
        self.constraints.push(format!("PRIMARY KEY ({})", columns.join(", ")));
        self
    }

    /// Add a foreign key constraint
    pub fn foreign_key(&mut self, column: &str, references_table: &str, references_column: &str) -> &mut Self {
        self.constraints.push(format!(
            "FOREIGN KEY ({}) REFERENCES {} ({})",
            column, references_table, references_column
        ));
        self
    }

    /// Add a unique constraint
    pub fn unique(&mut self, columns: &[&str]) -> &mut Self {
        self.constraints.push(format!("UNIQUE ({})", columns.join(", ")));
        self
    }

    /// Build the CREATE TABLE SQL
    pub fn to_sql(&self) -> String {
        let mut parts = self.columns.clone();
        parts.extend(self.constraints.clone());
        
        format!(
            "CREATE TABLE {} (\n    {}\n);",
            self.table_name,
            parts.join(",\n    ")
        )
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
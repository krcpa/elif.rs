//! Migration Manager - File system operations for migrations
//!
//! Handles creating, loading, and parsing migration files from the filesystem.

use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::error::{OrmError, OrmResult};
use super::definitions::{Migration, MigrationConfig};

/// Migration manager for creating and loading migrations
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

    /// Get the configuration
    pub fn config(&self) -> &MigrationConfig {
        &self.config
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

        // Extract ID and name from filename (format: YYYYMMDD_HHMMSS_name or timestamp_name)
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() < 2 {
            return Err(OrmError::Migration("Migration filename must follow format: timestamp_name".to_string()));
        }

        let id = filename.to_string();
        let name = if parts.len() >= 3 && parts[0].len() == 8 && parts[1].len() == 6 {
            // Handle format: YYYYMMDD_HHMMSS_name
            parts[2..].join("_").replace('_', " ")
        } else {
            // Handle format: timestamp_name
            parts[1..].join("_").replace('_', " ")
        };

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

    /// Split SQL statements for execution using proper SQL parsing
    pub fn split_sql_statements(&self, sql: &str) -> OrmResult<Vec<String>> {
        let dialect = GenericDialect {};
        let mut statements = Vec::new();
        
        // Parse all statements from the SQL string
        match Parser::parse_sql(&dialect, sql) {
            Ok(parsed_statements) => {
                for stmt in parsed_statements {
                    statements.push(format!("{};", stmt));
                }
                Ok(statements)
            }
            Err(e) => {
                // If parsing fails, fall back to the original naive approach with a warning
                tracing::warn!("SQL parsing failed, using naive semicolon splitting: {}", e);
                let naive_statements = sql.split(';')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("{};", s))
                    .collect();
                Ok(naive_statements)
            }
        }
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

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}
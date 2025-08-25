//! Schema Builder - DSL for creating database schema changes
//!
//! Provides a fluent interface for building SQL schema modification statements
//! commonly used in migrations.

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
        self.statements
            .push(format!("DROP TABLE IF EXISTS {};", table_name));
        self
    }

    /// Add a column to existing table
    pub fn add_column(
        &mut self,
        table_name: &str,
        column_name: &str,
        column_type: &str,
    ) -> &mut Self {
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
    pub fn create_index(
        &mut self,
        table_name: &str,
        column_names: &[&str],
        index_name: Option<&str>,
    ) -> &mut Self {
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
        self.statements
            .push(format!("DROP INDEX IF EXISTS {};", index_name));
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
    pub fn new(table_name: &str) -> Self {
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
        self.columns
            .push(format!("{} UUID DEFAULT gen_random_uuid()", name));
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
        self.columns
            .push("created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP".to_string());
        self.columns
            .push("updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP".to_string());
        self
    }

    /// Add a primary key constraint
    pub fn primary_key(&mut self, columns: &[&str]) -> &mut Self {
        self.constraints
            .push(format!("PRIMARY KEY ({})", columns.join(", ")));
        self
    }

    /// Add a foreign key constraint
    pub fn foreign_key(
        &mut self,
        column: &str,
        references_table: &str,
        references_column: &str,
    ) -> &mut Self {
        self.constraints.push(format!(
            "FOREIGN KEY ({}) REFERENCES {} ({})",
            column, references_table, references_column
        ));
        self
    }

    /// Add a unique constraint
    pub fn unique(&mut self, columns: &[&str]) -> &mut Self {
        self.constraints
            .push(format!("UNIQUE ({})", columns.join(", ")));
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
}

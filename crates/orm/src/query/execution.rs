//! Query Builder execution for Model types

use sqlx::Row;

use crate::error::ModelResult;
use crate::model::Model;
use super::builder::QueryBuilder;

// Implement specialized methods for Model-typed query builders
impl<M: Model> QueryBuilder<M> {
    /// Execute query and return models
    pub async fn get(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Vec<M>> {
        let sql = self.to_sql();
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await?;

        let mut models = Vec::new();
        for row in rows {
            models.push(M::from_row(&row)?);
        }

        Ok(models)
    }
    
    /// Execute query with chunking for large datasets
    pub async fn chunk<F>(
        self, 
        pool: &sqlx::Pool<sqlx::Postgres>, 
        chunk_size: i64,
        mut callback: F
    ) -> ModelResult<()>
    where
        F: FnMut(Vec<M>) -> Result<(), crate::error::ModelError>,
    {
        let mut offset = 0;
        loop {
            let chunk_query = self.clone()
                .limit(chunk_size)
                .offset(offset);
                
            let chunk = chunk_query.get(pool).await?;
            
            if chunk.is_empty() {
                break;
            }
            
            callback(chunk)?;
            offset += chunk_size;
        }
        
        Ok(())
    }
    
    /// Execute query and return raw SQL results (for complex aggregations)
    pub async fn get_raw(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Vec<serde_json::Value>> {
        let sql = self.to_sql();
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let mut json_row = serde_json::Map::new();
            
            // Convert PostgreSQL row to JSON
            // This is a simplified implementation
            for i in 0..row.len() {
                if let Ok(column) = row.try_get::<Option<String>, _>(i) {
                    let column_name = format!("column_{}", i); // Placeholder - real implementation would get actual column names
                    json_row.insert(column_name, serde_json::Value::String(column.unwrap_or_default()));
                }
            }
            
            results.push(serde_json::Value::Object(json_row));
        }
        
        Ok(results)
    }

    /// Execute query and return first model
    pub async fn first(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Option<M>> {
        let query = self.limit(1);
        let mut results = query.get(pool).await?;
        Ok(results.pop())
    }

    /// Execute query and return first model or error
    pub async fn first_or_fail(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<M> {
        self.first(pool)
            .await?
            .ok_or_else(|| crate::error::ModelError::NotFound(M::table_name().to_string()))
    }

    /// Count query results
    pub async fn count(mut self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<i64> {
        self.select_fields = vec!["COUNT(*)".to_string()];
        let sql = self.to_sql();
        
        let row = sqlx::query(&sql)
            .fetch_one(pool)
            .await?;

        let count: i64 = row.try_get(0)?;
        Ok(count)
    }
    
    /// Execute aggregation query and return single result
    pub async fn aggregate(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Option<serde_json::Value>> {
        let sql = self.to_sql();
        
        let row_opt = sqlx::query(&sql)
            .fetch_optional(pool)
            .await?;
            
        if let Some(row) = row_opt {
            // For aggregations, typically return the first column
            if let Ok(result) = row.try_get::<Option<i64>, _>(0) {
                return Ok(Some(serde_json::Value::Number(serde_json::Number::from(result.unwrap_or(0)))));
            } else if let Ok(result) = row.try_get::<Option<f64>, _>(0) {
                return Ok(Some(serde_json::Number::from_f64(result.unwrap_or(0.0)).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null)));
            } else if let Ok(result) = row.try_get::<Option<String>, _>(0) {
                return Ok(Some(serde_json::Value::String(result.unwrap_or_default())));
            }
        }
        
        Ok(None)
    }
}
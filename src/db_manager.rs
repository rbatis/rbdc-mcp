//! Database connection manager
//! 
//! Responsible for managing different types of database connections

use anyhow::{Ok, Result, anyhow};
use rbdc::db::{Connection, Driver};
use rbdc::pool::{ConnectionManager, Pool};
use rbdc_pool_fast::FastPool;
use rbs::Value;
use std::sync::Arc;
use std::time::Duration;

/// Supported database types
#[derive(Debug, Clone)]
pub enum DatabaseType {
    SQLite,
    MySQL,
    PostgreSQL,
    MSSQL,
}

impl DatabaseType {
    pub fn from_url(url: &str) -> Result<Self> {
        if url.starts_with("sqlite://") {
            Ok(DatabaseType::SQLite)
        } else if url.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
        } else if url.starts_with("pg://") || url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(DatabaseType::PostgreSQL)
        } else if url.starts_with("mssql://") || url.starts_with("sqlserver://") || url.starts_with("jdbc:sqlserver://") {
            Ok(DatabaseType::MSSQL)
        } else {
            Err(anyhow!("Unsupported database URL format: {}", url))
        }
    }
}

/// Database connection manager
pub struct DatabaseManager {
    pool: Arc<FastPool>,
    db_type: DatabaseType,
}


impl DatabaseManager {
    /// Create a new database manager
    pub fn new(url: &str) -> Result<Self> {
        log::debug!("Creating DatabaseManager with URL: {}", url);
        let db_type = DatabaseType::from_url(url)?;
        log::debug!("Detected database type: {:?}", db_type);

        let driver: Box<dyn Driver> = match db_type {
            DatabaseType::SQLite => Box::new(rbdc_sqlite::SqliteDriver {}),
            DatabaseType::MySQL => Box::new(rbdc_mysql::MysqlDriver {}),
            DatabaseType::PostgreSQL => Box::new(rbdc_pg::PgDriver {}),
            DatabaseType::MSSQL => Box::new(rbdc_mssql::MssqlDriver {}),
        };

        let manager = ConnectionManager::new(driver, url.as_ref())?;
        let pool = FastPool::new(manager)?;
        
        Ok(Self {
            pool: Arc::new(pool),
            db_type,
        })
    }

    /// Configure connection pool parameters
    pub async fn configure_pool(&self, max_connections: u64, timeout_seconds: u64) {
        self.pool.set_max_open_conns(max_connections).await;
        self.pool.set_timeout(Some(Duration::from_secs(timeout_seconds))).await;
    }

    /// Execute query and return result set
    pub async fn execute_query(&self, sql: &str, params: Vec<Value>) -> Result<Value> {
        let mut conn = self.pool.get().await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;
        let result = conn.get_values(sql, params).await
            .map_err(|e| anyhow!("Query execution failed: {}", e))?;
        Ok(result)
    }

    /// Execute modification operations (INSERT, UPDATE, DELETE, etc.)
    pub async fn execute_modification(&self, sql: &str, params: Vec<Value>) -> Result<serde_json::Value> {
        let mut conn = self.pool.get().await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;
            
        let result = conn.exec(sql, params).await
            .map_err(|e| anyhow!("Modification operation failed: {}", e))?;
            
        // Return JSON representation of operation result
        Ok(serde_json::json!({
            "rows_affected": result.rows_affected,
            "last_insert_id": result.last_insert_id
        }))
    }

    /// Get database type
    pub fn database_type(&self) -> &DatabaseType {
        &self.db_type
    }

    /// Get connection pool state
    pub async fn get_pool_state(&self) -> serde_json::Value {
        let state = self.pool.state().await;
        let mut result = serde_json::json!(state);
        // Add database type information
        if let Some(obj) = result.as_object_mut() {
            obj.insert("database_type".to_string(), serde_json::json!(format!("{:?}", self.database_type())));
        }
        result
    }

    /// Test database connection
    pub async fn test_connection(&self) -> Result<()> {
        let mut conn = self.pool.get().await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;
            
        conn.ping().await
            .map_err(|e| anyhow!("Database connection test failed: {}", e))?;
            
        Ok(())
    }
} 
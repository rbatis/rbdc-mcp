//! Database connection manager
//!
//! Responsible for managing different types of database connections

use crate::read_only::is_read_only_sql;
use anyhow::{anyhow, Result};
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
    Mssql,
    DuckDB,
    Turso,
}

impl DatabaseType {
    pub fn from_url(url: &str) -> Result<Self> {
        if url.starts_with("sqlite://") {
            Ok(DatabaseType::SQLite)
        } else if url.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
        } else if url.starts_with("pg://")
            || url.starts_with("postgres://")
            || url.starts_with("postgresql://")
        {
            Ok(DatabaseType::PostgreSQL)
        } else if url.starts_with("mssql://")
            || url.starts_with("sqlserver://")
            || url.starts_with("jdbc:sqlserver://")
        {
            Ok(DatabaseType::Mssql)
        } else if url.starts_with("duckdb://") {
            Ok(DatabaseType::DuckDB)
        } else if url.starts_with("turso://") || url.starts_with("libsql://") {
            Ok(DatabaseType::Turso)
        } else {
            Err(anyhow!("Unsupported database URL format: {}", url))
        }
    }
}

/// Database connection manager
pub struct DatabaseManager {
    pool: Arc<FastPool>,
    db_type: DatabaseType,
    read_only: bool,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(url: &str, read_only: bool) -> Result<Self> {
        log::debug!("Creating DatabaseManager with URL: {}", url);
        let db_type = DatabaseType::from_url(url)?;
        log::debug!("Detected database type: {:?}", db_type);

        let driver: Box<dyn Driver> = match db_type {
            DatabaseType::SQLite => {
                #[cfg(feature = "sqlite")]
                {
                    Box::new(rbdc_sqlite::SqliteDriver {})
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(anyhow!(
                        "SQLite driver not included. Rebuild with: cargo install --features sqlite"
                    ));
                }
            }
            DatabaseType::MySQL => {
                #[cfg(feature = "mysql")]
                {
                    Box::new(rbdc_mysql::MysqlDriver {})
                }
                #[cfg(not(feature = "mysql"))]
                {
                    return Err(anyhow!(
                        "MySQL driver not included. Rebuild with: cargo install --features mysql"
                    ));
                }
            }
            DatabaseType::PostgreSQL => {
                #[cfg(feature = "postgres")]
                {
                    Box::new(rbdc_pg::PgDriver {})
                }
                #[cfg(not(feature = "postgres"))]
                {
                    return Err(anyhow!(
                        "PostgreSQL driver not included. Rebuild with: cargo install --features postgres"
                    ));
                }
            }
            DatabaseType::Mssql => {
                #[cfg(feature = "mssql")]
                {
                    Box::new(rbdc_mssql::MssqlDriver {})
                }
                #[cfg(not(feature = "mssql"))]
                {
                    return Err(anyhow!(
                        "MSSQL driver not included. Rebuild with: cargo install --features mssql"
                    ));
                }
            }
            DatabaseType::DuckDB => {
                #[cfg(feature = "duckdb")]
                {
                    Box::new(rbdc_duckdb::DuckDbDriver {})
                }
                #[cfg(not(feature = "duckdb"))]
                {
                    return Err(anyhow!(
                        "DuckDB driver not included. Rebuild with: cargo install --features duckdb"
                    ));
                }
            }
            DatabaseType::Turso => {
                #[cfg(feature = "turso")]
                {
                    Box::new(rbdc_turso::TursoDriver {})
                }
                #[cfg(not(feature = "turso"))]
                {
                    return Err(anyhow!(
                        "Turso driver not included. Rebuild with: cargo install --features turso"
                    ));
                }
            }
        };

        let manager = ConnectionManager::new(driver, url)?;
        let pool = FastPool::new(manager)?;

        Ok(Self {
            pool: Arc::new(pool),
            db_type,
            read_only,
        })
    }

    /// Configure connection pool parameters
    pub async fn configure_pool(&self, max_connections: u64, timeout_seconds: u64) {
        self.pool.set_max_open_conns(max_connections).await;
        self.pool
            .set_timeout(Some(Duration::from_secs(timeout_seconds)))
            .await;
    }

    /// Execute query and return result set
    pub async fn execute_query(&self, sql: &str, params: Vec<Value>) -> Result<Value> {
        if !is_read_only_sql(sql) {
            return Err(anyhow!("Read-only query validation failed"));
        }

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;
        let result = conn
            .exec_decode(sql, params)
            .await
            .map_err(|e| anyhow!("Query execution failed: {}", e))?;
        Ok(result)
    }

    /// Execute modification operations (INSERT, UPDATE, DELETE, etc.)
    pub async fn execute_modification(
        &self,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<serde_json::Value> {
        if self.read_only {
            return Err(anyhow!(
                "Read-only mode blocks SQL modifications."
            ));
        }

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let result = conn
            .exec(sql, params)
            .await
            .map_err(|e| anyhow!("Modification operation failed: {}", e))?;

        Ok(serde_json::json!({
            "rows_affected": result.rows_affected,
            "last_insert_id": result.last_insert_id
        }))
    }

    /// Get database type
    pub fn database_type(&self) -> &DatabaseType {
        &self.db_type
    }

    /// Whether server read-only mode is enabled
    pub fn read_only_enabled(&self) -> bool {
        self.read_only
    }

    /// Get connection pool state
    pub async fn get_pool_state(&self) -> serde_json::Value {
        let state = self.pool.state().await;
        let mut result = serde_json::json!(state);
        if let Some(obj) = result.as_object_mut() {
            obj.insert(
                "database_type".to_string(),
                serde_json::json!(format!("{:?}", self.database_type())),
            );
            obj.insert("read_only".to_string(), serde_json::json!(self.read_only));
        }
        result
    }

    /// Test database connection
    pub async fn test_connection(&self) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        conn.ping()
            .await
            .map_err(|e| anyhow!("Database connection test failed: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DatabaseType;

    #[test]
    fn from_url_accepts_postgres_scheme() {
        let db_type = DatabaseType::from_url("postgres://user:pass@localhost/db").unwrap();
        assert!(matches!(db_type, DatabaseType::PostgreSQL));
    }
}

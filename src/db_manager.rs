//! Database connection manager
//!
//! Holds a thread-safe `SyncHashMap<String, FastPool>` so the AI client can
//! register additional database URLs at runtime. The CLI-provided URL is
//! registered under the [`DEFAULT_DB_ALIAS`] alias.

use crate::read_only::is_read_only_sql;
use anyhow::{anyhow, Result};
use dark_std::sync::SyncHashMap;
use rbdc::db::{Connection, Driver};
use rbdc::pool::{ConnectionManager, Pool};
use rbdc_pool_fast::FastPool;
use rbs::Value;
use std::sync::Arc;
use std::time::Duration;

/// Default alias used for the database URL provided via the CLI.
pub const DEFAULT_DB_ALIAS: &str = "default";

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

/// Per-alias pool bookkeeping stored in the multi-database registry.
struct PoolEntry {
    pool: Arc<FastPool>,
    db_type: DatabaseType,
    url: String,
}

/// Database connection manager.
///
/// Stores a [`SyncHashMap`] of `alias -> FastPool` so the AI client can
/// dynamically add and remove database connections through MCP tools. The
/// alias provided via the CLI is registered under [`DEFAULT_DB_ALIAS`].
pub struct DatabaseManager {
    pools: Arc<SyncHashMap<String, PoolEntry>>,
    /// Pool sizing applied to pools registered after construction.
    max_connections: u64,
    timeout_seconds: u64,
    read_only: bool,
}

impl DatabaseManager {
    /// Build the driver for a database type, returning a clear error if the
    /// matching cargo feature is not enabled.
    fn build_driver(db_type: &DatabaseType) -> Result<Box<dyn Driver>> {
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
        Ok(driver)
    }

    /// Create a pool for `url` using the manager's current pool sizing.
    async fn build_pool(&self, url: &str) -> Result<PoolEntry> {
        let db_type = DatabaseType::from_url(url)?;
        let driver = Self::build_driver(&db_type)?;
        let manager = ConnectionManager::new(driver, url)?;
        let pool = FastPool::new(manager)?;
        pool.set_max_open_conns(self.max_connections).await;
        pool.set_timeout(Some(Duration::from_secs(self.timeout_seconds)))
            .await;
        Ok(PoolEntry {
            pool: Arc::new(pool),
            db_type,
            url: url.to_string(),
        })
    }

    /// Create a new database manager. The CLI URL is validated eagerly so
    /// the process fails fast on a bad URL; the actual pool is created in
    /// [`DatabaseManager::configure_pool`] so pool sizing is known up front
    /// and runtime registration reuses the same code path.
    pub fn new(url: &str, read_only: bool) -> Result<Self> {
        log::debug!("Creating DatabaseManager with URL: {}", url);
        let db_type = DatabaseType::from_url(url)?;
        log::debug!("Detected database type: {:?}", db_type);
        let _ = db_type;
        Ok(Self {
            pools: Arc::new(SyncHashMap::new()),
            max_connections: 1,
            timeout_seconds: 30,
            read_only,
        })
    }

    /// Configure pool sizing and create the default pool for `url` under
    /// the [`DEFAULT_DB_ALIAS`] alias. Subsequent calls to
    /// [`DatabaseManager::add_database`] use the same sizing.
    pub async fn configure_pool(
        &mut self,
        url: &str,
        max_connections: u64,
        timeout_seconds: u64,
    ) -> Result<()> {
        self.max_connections = max_connections;
        self.timeout_seconds = timeout_seconds;
        if self.pools.contains_key(&DEFAULT_DB_ALIAS.to_string()) {
            return Ok(());
        }
        let entry = self.build_pool(url).await?;
        self.pools
            .insert(DEFAULT_DB_ALIAS.to_string(), entry);
        log::info!("Registered default database alias for {}", url);
        Ok(())
    }

    /// Register a new database alias. Returns an error if the alias already
    /// exists or the URL is unsupported.
    pub async fn add_database(&self, alias: &str, url: &str) -> Result<()> {
        let alias = alias.trim();
        if alias.is_empty() {
            return Err(anyhow!("Database alias must not be empty"));
        }
        if self.pools.contains_key(&alias.to_string()) {
            return Err(anyhow!("Database alias '{}' already exists", alias));
        }
        let entry = self.build_pool(url).await?;
        self.pools.insert(alias.to_string(), entry);
        log::info!("Registered database alias '{}' for {}", alias, url);
        Ok(())
    }

    /// Remove a database alias. The [`DEFAULT_DB_ALIAS`] alias cannot be
    /// removed so the CLI-provided database always remains reachable.
    pub fn remove_database(&self, alias: &str) -> Result<()> {
        let alias = alias.trim();
        if alias == DEFAULT_DB_ALIAS {
            return Err(anyhow!(
                "Cannot remove the '{}' alias",
                DEFAULT_DB_ALIAS
            ));
        }
        if self.pools.remove(&alias.to_string()).is_none() {
            return Err(anyhow!("Database alias '{}' does not exist", alias));
        }
        log::info!("Removed database alias '{}'", alias);
        Ok(())
    }

    /// Resolve an alias to its pool entry. Falls back to [`DEFAULT_DB_ALIAS`].
    fn resolve(&self, alias: Option<&str>) -> Result<&PoolEntry> {
        let key = alias.unwrap_or(DEFAULT_DB_ALIAS);
        self.pools
            .get(&key.to_string())
            .ok_or_else(|| anyhow!("Unknown database alias '{}'", key))
    }

    /// List registered database aliases with their URL and type.
    pub fn list_databases(&self) -> Vec<serde_json::Value> {
        let mut out: Vec<serde_json::Value> = Vec::new();
        for (alias, entry) in self.pools.iter() {
            out.push(serde_json::json!({
                "alias": alias,
                "url": entry.url,
                "database_type": format!("{:?}", entry.db_type),
            }));
        }
        out.sort_by(|a, b| {
            let aa = a.get("alias").and_then(|v| v.as_str()).unwrap_or("");
            let bb = b.get("alias").and_then(|v| v.as_str()).unwrap_or("");
            if aa == DEFAULT_DB_ALIAS {
                std::cmp::Ordering::Less
            } else if bb == DEFAULT_DB_ALIAS {
                std::cmp::Ordering::Greater
            } else {
                aa.cmp(bb)
            }
        });
        out
    }

    /// Execute a read-only query against the named alias (or `default`).
    pub async fn execute_query(
        &self,
        alias: Option<&str>,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<Value> {
        if !is_read_only_sql(sql) {
            return Err(anyhow!("Read-only query validation failed"));
        }
        let entry = self.resolve(alias)?;
        let mut conn = entry
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

    /// Execute a write operation (INSERT/UPDATE/DELETE, etc.) against the
    /// named alias (or `default`).
    pub async fn execute_modification(
        &self,
        alias: Option<&str>,
        sql: &str,
        params: Vec<Value>,
    ) -> Result<serde_json::Value> {
        if self.read_only {
            return Err(anyhow!("Read-only mode blocks SQL modifications."));
        }
        let entry = self.resolve(alias)?;
        let mut conn = entry
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

    /// Whether server read-only mode is enabled.
    pub fn read_only_enabled(&self) -> bool {
        self.read_only
    }

    /// Get connection pool state for the named alias (or `default`).
    pub async fn get_pool_state(&self, alias: Option<&str>) -> Result<serde_json::Value> {
        let entry = self.resolve(alias)?;
        let state = entry.pool.state().await;
        let mut result = serde_json::json!(state);
        if let Some(obj) = result.as_object_mut() {
            obj.insert(
                "database_type".to_string(),
                serde_json::json!(format!("{:?}", entry.db_type)),
            );
            obj.insert("read_only".to_string(), serde_json::json!(self.read_only));
            obj.insert(
                "alias".to_string(),
                serde_json::json!(alias.unwrap_or(DEFAULT_DB_ALIAS)),
            );
        }
        Ok(result)
    }

    /// Test the connection for the named alias (or `default`).
    pub async fn test_connection(&self, alias: Option<&str>) -> Result<()> {
        let entry = self.resolve(alias)?;
        let mut conn = entry
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

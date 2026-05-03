//! Database connection manager
//!
//! Responsible for managing different types of database connections

use crate::sql_guard::is_read_only_sql;
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

        validate_read_only_configuration(&db_type, url, read_only)?;

        let driver: Box<dyn Driver> = match db_type {
            DatabaseType::SQLite => Box::new(rbdc_sqlite::SqliteDriver {}),
            DatabaseType::MySQL => Box::new(rbdc_mysql::MysqlDriver {}),
            DatabaseType::PostgreSQL => Box::new(rbdc_pg::PgDriver {}),
            DatabaseType::Mssql => Box::new(rbdc_mssql::MssqlDriver {}),
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
                "Read-only mode blocks SQL modifications. Use a database connection with read-only access and submit only read-only queries."
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

    /// Whether server read-only mode is enabled
    pub fn read_only_enabled(&self) -> bool {
        self.read_only
    }

    /// Human-readable startup notice for database-specific read-only expectations
    pub fn read_only_startup_notice(&self) -> Option<&'static str> {
        if !self.read_only {
            return None;
        }

        match self.db_type {
            DatabaseType::SQLite => Some(
                "Read-only mode enabled with SQLite URI validation. Keep using a sqlite URL configured with mode=ro."
            ),
            DatabaseType::MySQL => Some(
                "Read-only mode enabled. For MySQL, the real safety boundary is a database user without write privileges."
            ),
            DatabaseType::PostgreSQL => Some(
                "Read-only mode enabled. For PostgreSQL, the real safety boundary is a read-only role or equivalent database privileges."
            ),
            DatabaseType::Mssql => Some(
                "Read-only mode enabled. For MSSQL, the real safety boundary is a login/user with reader-only permissions."
            ),
        }
    }

    /// Validate that the connected session is provably read-only.
    pub async fn validate_read_only_session(&self) -> Result<()> {
        if !self.read_only {
            return Ok(());
        }

        match self.db_type {
            DatabaseType::SQLite => self.validate_sqlite_read_only_session().await,
            DatabaseType::MySQL => self.validate_mysql_read_only_session().await,
            DatabaseType::PostgreSQL => self.validate_postgres_read_only_session().await,
            DatabaseType::Mssql => self.validate_mssql_read_only_session().await,
        }
    }

    /// Get connection pool state
    pub async fn get_pool_state(&self) -> serde_json::Value {
        let state = self.pool.state().await;
        let mut result = serde_json::json!(state);
        // Add database type information
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

    async fn validate_sqlite_read_only_session(&self) -> Result<()> {
        // SQLite read-only mode is validated at startup by requiring mode=ro in the connection URI.
        Ok(())
    }

    async fn validate_mysql_read_only_session(&self) -> Result<()> {
        let value = match self
            .fetch_session_scalar("SELECT @@session.transaction_read_only AS read_only")
            .await
        {
            Ok(value) => value,
            Err(_) => {
                self.fetch_session_scalar("SELECT @@session.tx_read_only AS read_only")
                    .await?
            }
        };

        if scalar_value_is_enabled(&value) {
            Ok(())
        } else {
            Err(anyhow!(
                "Read-only mode requires a verifiably read-only MySQL session. Expected @@session.transaction_read_only = 1."
            ))
        }
    }

    async fn validate_postgres_read_only_session(&self) -> Result<()> {
        let value = self
            .fetch_session_scalar("SHOW transaction_read_only")
            .await?;

        if scalar_value_is_enabled(&value) {
            Ok(())
        } else {
            Err(anyhow!(
                "Read-only mode requires a verifiably read-only PostgreSQL session. Expected transaction_read_only = on."
            ))
        }
    }

    async fn validate_mssql_read_only_session(&self) -> Result<()> {
        let value = self
            .fetch_session_scalar(
                "SELECT CAST(DATABASEPROPERTYEX(DB_NAME(), 'Updateability') AS NVARCHAR(60)) AS updateability",
            )
            .await?;

        if scalar_value_matches(&value, &["READ_ONLY"]) {
            Ok(())
        } else {
            Err(anyhow!(
                "Read-only mode requires a verifiably read-only MSSQL database. Expected DATABASEPROPERTYEX(DB_NAME(), 'Updateability') = READ_ONLY."
            ))
        }
    }

    async fn fetch_session_scalar(&self, sql: &str) -> Result<Value> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let rows = conn
            .exec_decode(sql, vec![])
            .await
            .map_err(|e| anyhow!("Read-only session validation query failed: {}", e))?;

        first_scalar_from_rows(&rows).ok_or_else(|| {
            anyhow!(
                "Read-only session validation query returned an unexpected result shape: {}",
                rows
            )
        })
    }
}

fn validate_read_only_configuration(
    db_type: &DatabaseType,
    url: &str,
    read_only: bool,
) -> Result<()> {
    if !read_only {
        return Ok(());
    }

    match db_type {
        DatabaseType::SQLite => {
            if !sqlite_url_is_explicitly_read_only(url) {
                return Err(anyhow!(
                    "Read-only mode for SQLite requires an explicit read-only URI, for example sqlite://path/to/database.db?mode=ro"
                ));
            }
        }
        DatabaseType::MySQL | DatabaseType::PostgreSQL | DatabaseType::Mssql => {}
    }

    Ok(())
}

fn sqlite_url_is_explicitly_read_only(url: &str) -> bool {
    let lowercase = url.to_ascii_lowercase();
    lowercase.contains("?mode=ro") || lowercase.contains("&mode=ro")
}

fn first_scalar_from_rows(rows: &Value) -> Option<Value> {
    let first_row = rows.as_array()?.first()?;

    if let Some(map) = first_row.as_map() {
        if let Some((_, value)) = map.into_iter().next() {
            return Some(value.clone());
        }
    }

    Some(first_row.clone())
}

fn scalar_value_is_enabled(value: &Value) -> bool {
    if let Some(flag) = value.as_bool() {
        return flag;
    }

    if let Some(number) = value.as_i64() {
        return number != 0;
    }

    if let Some(text) = value.as_str() {
        return matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "1" | "on" | "true" | "read_only"
        );
    }

    false
}

fn scalar_value_matches(value: &Value, allowed: &[&str]) -> bool {
    value.as_str().is_some_and(|text| {
        allowed
            .iter()
            .any(|candidate| text.trim().eq_ignore_ascii_case(candidate))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        first_scalar_from_rows, scalar_value_is_enabled, scalar_value_matches,
        sqlite_url_is_explicitly_read_only, validate_read_only_configuration, DatabaseType,
    };
    use rbs::{value::map::ValueMap, Value};

    #[test]
    fn from_url_accepts_postgres_scheme() {
        let db_type = DatabaseType::from_url("postgres://user:pass@localhost/db").unwrap();
        assert!(matches!(db_type, DatabaseType::PostgreSQL));
    }

    #[test]
    fn sqlite_read_only_url_detection_accepts_query_parameter() {
        assert!(sqlite_url_is_explicitly_read_only(
            "sqlite://./db.sqlite?mode=ro"
        ));
        assert!(sqlite_url_is_explicitly_read_only(
            "sqlite://./db.sqlite?cache=shared&mode=ro"
        ));
    }

    #[test]
    fn sqlite_read_only_url_detection_rejects_plain_sqlite_url() {
        assert!(!sqlite_url_is_explicitly_read_only("sqlite://./db.sqlite"));
    }

    #[test]
    fn validate_read_only_configuration_requires_explicit_sqlite_mode() {
        let db_type = DatabaseType::SQLite;
        let result = validate_read_only_configuration(&db_type, "sqlite://./db.sqlite", true);
        assert!(result.is_err());
    }

    #[test]
    fn validate_read_only_configuration_allows_non_sqlite_with_read_only_flag() {
        let db_type = DatabaseType::MySQL;
        let result =
            validate_read_only_configuration(&db_type, "mysql://user:pass@localhost/db", true);
        assert!(result.is_ok());
    }

    #[test]
    fn first_scalar_from_rows_reads_first_map_value() {
        let mut map = ValueMap::new();
        map["read_only"] = Value::Bool(true);
        let rows = Value::Array(vec![Value::Map(map)]);

        let value = first_scalar_from_rows(&rows).unwrap();
        assert_eq!(value.as_bool(), Some(true));
    }

    #[test]
    fn scalar_value_is_enabled_accepts_bool_number_and_text() {
        assert!(scalar_value_is_enabled(&Value::Bool(true)));
        assert!(scalar_value_is_enabled(&Value::I64(1)));
        assert!(scalar_value_is_enabled(&Value::String("on".to_string())));
        assert!(!scalar_value_is_enabled(&Value::String("off".to_string())));
    }

    #[test]
    fn scalar_value_matches_compares_case_insensitively() {
        assert!(scalar_value_matches(
            &Value::String("READ_ONLY".to_string()),
            &["read_only"]
        ));
        assert!(!scalar_value_matches(
            &Value::String("READ_WRITE".to_string()),
            &["read_only"]
        ));
    }
}

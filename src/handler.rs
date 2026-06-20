use crate::db_manager::DatabaseManager;
use crate::read_only::is_read_only_sql;
use std::sync::Arc;

use rmcp::{
    handler::server::wrapper::Parameters,
    model::*,
    schemars,
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

#[derive(Clone)]
pub struct RbdcDatabaseHandler {
    db_manager: Arc<DatabaseManager>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SqlQueryParams {
    /// Optional database alias. Defaults to the CLI-provided "default" alias
    /// when omitted. Use `list_databases` to see registered aliases.
    #[serde(default)]
    alias: Option<String>,
    /// SQL query statement to execute
    sql: String,
    /// SQL parameter array, optional
    #[serde(default)]
    params: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SqlExecParams {
    /// Optional database alias. Defaults to the CLI-provided "default" alias
    /// when omitted. Use `list_databases` to see registered aliases.
    #[serde(default)]
    alias: Option<String>,
    /// SQL modification statement to execute
    sql: String,
    /// SQL parameter array, optional
    #[serde(default)]
    params: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DbStatusParams {
    /// Optional database alias. Defaults to the CLI-provided "default" alias
    /// when omitted.
    #[serde(default)]
    alias: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TestConnectionParams {
    /// Optional database alias. Defaults to the CLI-provided "default" alias
    /// when omitted.
    #[serde(default)]
    alias: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddDatabaseParams {
    /// Unique alias used to refer to this database from `sql_query` /
    /// `sql_exec`. The reserved alias "default" cannot be reused.
    alias: String,
    /// Database connection URL (sqlite://, mysql://, pg://, mssql://,
    /// duckdb://, turso://, libsql://).
    url: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RemoveDatabaseParams {
    /// Alias of the database to unregister. The "default" alias is reserved
    /// and cannot be removed.
    alias: String,
}

// Use tool_router macro to generate the tool router
#[tool_router]
impl RbdcDatabaseHandler {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
        }
    }

    fn convert_params(&self, params: &[serde_json::Value]) -> Result<Vec<rbs::Value>, McpError> {
        params
            .iter()
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|e| {
                    McpError::invalid_params(
                        format!("Failed to convert parameter: {}", e),
                        None,
                    )
                })
            })
            .collect()
    }

    #[tool(description = "Execute SQL query and return results")]
    async fn sql_query(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<SqlQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        if !is_read_only_sql(&params.sql) {
            return Err(McpError::invalid_params(
                "sql_query only accepts single read-only SQL statements".to_string(),
                None,
            ));
        }

        // Convert parameter types from serde_json::Value to rbs::Value
        let rbs_params = self.convert_params(&params.params)?;

        match self
            .db_manager
            .execute_query(params.alias.as_deref(), &params.sql, rbs_params)
            .await
        {
            Ok(results) => {
                let content = Content::json(results).map_err(|e| {
                    McpError::internal_error(format!("Result serialization failed: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![content]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("SQL query failed: {}", e),
                None,
            )),
        }
    }

    #[tool(description = "Execute SQL modification statements (INSERT/UPDATE/DELETE)")]
    async fn sql_exec(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<SqlExecParams>,
    ) -> Result<CallToolResult, McpError> {
        if self.db_manager.read_only_enabled() {
            return Err(McpError::invalid_params(
                "Read-only mode blocks SQL modifications. Configure the database itself for read-only access and use sql_query only for single read-only statements.".to_string(),
                None,
            ));
        }

        // Convert parameter types from serde_json::Value to rbs::Value
        let rbs_params = self.convert_params(&params.params)?;

        match self
            .db_manager
            .execute_modification(params.alias.as_deref(), &params.sql, rbs_params)
            .await
        {
            Ok(result) => {
                let content = Content::json(result).map_err(|e| {
                    McpError::internal_error(format!("Result serialization failed: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![content]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("SQL execution failed: {}", e),
                None,
            )),
        }
    }

    #[tool(description = "Get database connection pool status information")]
    async fn db_status(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<DbStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let status = self
            .db_manager
            .get_pool_state(params.alias.as_deref())
            .await;
        let status = status.map_err(|e| {
            McpError::internal_error(format!("Status retrieval failed: {}", e), None)
        })?;
        let content = Content::json(status).map_err(|e| {
            McpError::internal_error(format!("Status serialization failed: {}", e), None)
        })?;
        Ok(CallToolResult::success(vec![content]))
    }

    #[tool(description = "Test connectivity for a registered database. Pass `alias` to target a specific database; omit it to test the 'default' database.")]
    async fn test_connection(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<TestConnectionParams>,
    ) -> Result<CallToolResult, McpError> {
        let alias = params.alias.clone();
        match self
            .db_manager
            .test_connection(params.alias.as_deref())
            .await
        {
            Ok(()) => {
                let content = Content::json(serde_json::json!({
                    "alias": alias.unwrap_or_else(|| "default".to_string()),
                    "status": "ok"
                }))
                .map_err(|e| {
                    McpError::internal_error(
                        format!("Status serialization failed: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![content]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("Connection test failed: {}", e),
                None,
            )),
        }
    }

    #[tool(description = "Register a new database connection under `alias` and start a pool. Use `list_databases` to inspect currently registered databases.")]
    async fn add_database(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<AddDatabaseParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .db_manager
            .add_database(&params.alias, &params.url)
            .await
        {
            Ok(()) => {
                let content = Content::json(serde_json::json!({
                    "alias": params.alias,
                    "url": params.url,
                    "status": "registered"
                }))
                .map_err(|e| {
                    McpError::internal_error(
                        format!("Status serialization failed: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![content]))
            }
            Err(e) => Err(McpError::invalid_params(
                format!("Failed to register database: {}", e),
                None,
            )),
        }
    }

    #[tool(description = "Unregister a previously-added database alias. The reserved 'default' alias cannot be removed.")]
    async fn remove_database(
        &self,
        _context: RequestContext<RoleServer>,
        Parameters(params): Parameters<RemoveDatabaseParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.db_manager.remove_database(&params.alias) {
            Ok(()) => {
                let content = Content::json(serde_json::json!({
                    "alias": params.alias,
                    "status": "removed"
                }))
                .map_err(|e| {
                    McpError::internal_error(
                        format!("Status serialization failed: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![content]))
            }
            Err(e) => Err(McpError::invalid_params(
                format!("Failed to remove database: {}", e),
                None,
            )),
        }
    }

    #[tool(description = "List all registered database aliases along with their URL and detected type.")]
    async fn list_databases(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let list = self.db_manager.list_databases();
        let content = Content::json(list).map_err(|e| {
            McpError::internal_error(
                format!("Status serialization failed: {}", e),
                None,
            )
        })?;
        Ok(CallToolResult::success(vec![content]))
    }
}

#[tool_handler]
impl ServerHandler for RbdcDatabaseHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new(
            "RBDC MCP Server",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "RBDC database MCP server providing SQL query, execution and status check tools. sql_query accepts only single read-only SQL statements.",
        )
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}

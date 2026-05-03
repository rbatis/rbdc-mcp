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
    /// SQL query statement to execute
    sql: String,
    /// SQL parameter array, optional
    #[serde(default)]
    params: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SqlExecParams {
    /// SQL modification statement to execute
    sql: String,
    /// SQL parameter array, optional
    #[serde(default)]
    params: Vec<serde_json::Value>,
}

// Use tool_router macro to generate the tool router
#[tool_router]
impl RbdcDatabaseHandler {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
        }
    }

    fn convert_params(&self, params: &[serde_json::Value]) -> Vec<rbs::Value> {
        params
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
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
        let rbs_params = self.convert_params(&params.params);

        match self.db_manager.execute_query(&params.sql, rbs_params).await {
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
        let rbs_params = self.convert_params(&params.params);

        match self
            .db_manager
            .execute_modification(&params.sql, rbs_params)
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
    ) -> Result<CallToolResult, McpError> {
        let status = self.db_manager.get_pool_state().await;
        let content = Content::json(status).map_err(|e| {
            McpError::internal_error(format!("Status serialization failed: {}", e), None)
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

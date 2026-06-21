#[allow(dead_code)]

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

pub mod db_manager;
pub mod handler;
pub mod read_only;

use crate::db_manager::DatabaseManager;
use crate::handler::RbdcDatabaseHandler;
use rmcp::{transport::stdio, ServiceExt};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "rbdc-mcp")]
#[command(about = "RBDC MCP Server - Provides SQL query and modification tools")]
struct Args {
    /// Database connection URL. Can be omitted entirely — the server
    /// starts empty and databases can be added at runtime via the
    /// `add_database` MCP tool. Repeat the flag to register multiple
    /// databases at startup. The first URL becomes the `default` alias.
    #[arg(short, long, action = clap::ArgAction::Append)]
    database_url: Vec<String>,

    /// Alias for the corresponding `--database-url` in declaration order.
    /// If fewer aliases are provided than URLs, remaining databases are
    /// auto-named `db2`, `db3`, ...
    #[arg(long, action = clap::ArgAction::Append)]
    alias: Vec<String>,

    /// Maximum number of connections
    #[arg(long, default_value = "1")]
    max_connections: u64,

    /// Connection timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enforce read-only server mode (blocks sql_exec)
    #[arg(long, default_value_t = false)]
    read_only: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                args.log_level
                    .parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()),
            ),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!("Starting RBDC MCP Server");
    info!("Read-only mode: {}", args.read_only);

    // Create database manager (no connection yet). When URLs are provided,
    // the first is validated eagerly via `DatabaseManager::new`.
    let first_url = args.database_url.first().map(|s| s.as_str());
    let mut db_manager = DatabaseManager::new(first_url, args.read_only).map_err(|e| {
        error!("Failed to create database manager: {}", e);
        anyhow::Error::msg(e.to_string())
    })?;

    // Configure connection pool and register every startup database,
    // or just log that the server started without any database.
    if args.database_url.is_empty() {
        info!("No --database-url provided; server started empty. Use add_database to register databases at runtime.");
    } else {
        // Build the (alias, url) registration list. The first URL is always
        // `default`; any extra URLs use a user-supplied --alias or an auto
        // name (db2, db3, ...).
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(args.database_url.len());
        for (idx, url) in args.database_url.iter().enumerate() {
            let alias = if idx == 0 {
                db_manager::DEFAULT_DB_ALIAS.to_string()
            } else {
                args.alias
                    .get(idx - 1)
                    .cloned()
                    .unwrap_or_else(|| format!("db{}", idx + 1))
            };
            pairs.push((alias, url.clone()));
        }
        for (alias, url) in &pairs {
            info!("Will register alias '{}' -> {}", alias, url);
        }

        // Configure connection pool and register every startup database.
        for (idx, (alias, url)) in pairs.iter().enumerate() {
            if idx == 0 {
                if let Err(e) = db_manager
                    .configure_pool(url, args.max_connections, args.timeout)
                    .await
                {
                    error!("Failed to configure connection pool: {}", e);
                    return Err(anyhow::Error::msg(e.to_string()));
                }
            } else if let Err(e) = db_manager.add_database(alias, url).await {
                error!("Failed to register alias '{}': {}", alias, e);
                return Err(anyhow::Error::msg(e.to_string()));
            }
        }
    }

    let db_manager = Arc::new(db_manager);

    // Test DB connection in background (if a default database exists) —
    // do NOT block MCP startup. Claude Desktop's initialize request must
    // be answered within ~60s or it times out.
    {
        let db = Arc::clone(&db_manager);
        tokio::spawn(async move {
            match db.test_connection(None).await {
                Ok(()) => info!("Default database connection test successful"),
                Err(e) => error!("Default database connection test failed: {}", e),
            }
        });
    }

    // Start MCP server immediately so initialize is handled without delay
    let handler = RbdcDatabaseHandler::new(db_manager);

    info!("Starting RBDC MCP Server...");

    let service = handler.serve(stdio()).await.inspect_err(|e| {
        error!("Server startup failed: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

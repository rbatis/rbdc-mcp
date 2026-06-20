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
    /// Database connection URL
    #[arg(short, long)]
    database_url: String,

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
    info!("Database URL: {}", args.database_url);
    info!("Read-only mode: {}", args.read_only);

    // Create database manager (no connection yet)
    let mut db_manager = DatabaseManager::new(&args.database_url, args.read_only).map_err(|e| {
        error!("Failed to create database manager: {}", e);
        anyhow::Error::msg(e.to_string())
    })?;

    // Configure connection pool
    if let Err(e) = db_manager
        .configure_pool(&args.database_url, args.max_connections, args.timeout)
        .await
    {
        error!("Failed to configure connection pool: {}", e);
        return Err(anyhow::Error::msg(e.to_string()));
    }

    let db_manager = Arc::new(db_manager);

    // Test DB connection in background — do NOT block MCP startup.
    // Claude Desktop's initialize request must be answered within ~60s or it times out.
    {
        let db = Arc::clone(&db_manager);
        tokio::spawn(async move {
            match db.test_connection(None).await {
                Ok(()) => info!("Database connection test successful"),
                Err(e) => error!("Database connection test failed: {}", e),
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

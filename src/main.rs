use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{EnvFilter};

mod db_manager;
mod handler;
mod sql_guard;

use crate::db_manager::DatabaseManager;
use crate::handler::RbdcDatabaseHandler;
use rmcp::{ServiceExt, transport::stdio};

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
            EnvFilter::from_default_env()
                .add_directive(args.log_level.parse()
                    .unwrap_or_else(|_| tracing::Level::INFO.into()))
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!("Starting RBDC MCP Server");
    info!("Database URL: {}", args.database_url);
    info!("Read-only mode: {}", args.read_only);

    // Create database manager
    let db_manager = DatabaseManager::new(&args.database_url)
        .map_err(|e| {
            error!("Failed to create database manager: {}", e);
            anyhow::Error::msg(e.to_string())
        })?;
    
    // Configure connection pool
    db_manager.configure_pool(args.max_connections, args.timeout).await;
    
    // Test database connection
    db_manager.test_connection().await
        .map_err(|e| anyhow::Error::msg(format!("Database connection test failed: {}", e)))?;
    
    info!("Database connection test successful");

    // Create RBDC database handler
    let handler = RbdcDatabaseHandler::new(Arc::new(db_manager), args.read_only);

    info!("Starting RBDC MCP Server...");
    
    // Start server
    let service = handler.serve(stdio()).await.inspect_err(|e| {
        error!("Server startup failed: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}

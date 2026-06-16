# RBDC MCP Server

A database server based on [Model Context Protocol (MCP)](https://modelcontextprotocol.io), supporting SQLite, MySQL, PostgreSQL, MSSQL, DuckDB, and Turso databases.

**🇨🇳 中文文档 / Chinese Documentation**: [readme_cn.md](./readme_cn.md)

## Advantages

- **Multiple Database Support**: Seamlessly work with SQLite, MySQL, PostgreSQL, MSSQL, DuckDB, and Turso using a unified interface
- **AI Integration**: Native integration with Claude AI through the Model Context Protocol
- **Zero Configuration**: Automatic management of database connections and resources
- **Security**: Controlled access to your database through AI-driven natural language queries
- **Simplicity**: Use natural language to query and modify your database without writing SQL

## Installation

### 🚀 Method 1: Install via Cargo (Recommended)

**Prerequisites:** Install [Rust](https://rustup.rs/) first.

Choose the install command based on your needs:

```bash
# All drivers (default, ~10-15 minutes build)
cargo install --git https://github.com/rbatis/rbdc-mcp.git

# Minimal: SQLite only (fastest build, ~2-3 minutes)
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features sqlite

# Single driver (e.g., MySQL):
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features mysql

# Multiple drivers:
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features "mysql postgres"
```

**💡 Build speed tip:** If you only need one database (e.g., SQLite), add `--no-default-features --features sqlite` to skip compiling unused drivers, cutting build time from ~15 minutes to ~2 minutes.

#### Available Features

| Feature | Driver | Description |
|---------|--------|-------------|
| `sqlite` | `rbdc-sqlite` | SQLite support |
| `mysql` | `rbdc-mysql` | MySQL support |
| `postgres` | `rbdc-pg` | PostgreSQL support |
| `mssql` | `rbdc-mssql` | MSSQL/SQL Server support |
| `duckdb` | `rbdc-duckdb` | DuckDB support |
| `turso` | `rbdc-turso` | Turso/libsql support |
| `full` | *(all above)* | Enable all database drivers |

### 📦 Method 2: Download Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/rbatis/rbdc-mcp/releases):

| Platform | Download |
|----------|----------|
| **Windows (x64)** | `rbdc-mcp-windows-x86_64.exe` |
| **macOS (Intel)** | `rbdc-mcp-macos-x86_64` |
| **macOS (Apple Silicon)** | `rbdc-mcp-macos-aarch64` |
| **Linux (x64)** | `rbdc-mcp-linux-x86_64` |

After downloading, rename the file to `rbdc-mcp` (or `rbdc-mcp.exe` on Windows) and add it to your system PATH.

## 🔧 Agent Client Configuration

Configure rbdc-mcp in your MCP-compatible client by adding it to the MCP server list.

### Claude Desktop

**Configuration File Location:**
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

**Basic Configuration:**

```json
{
  "mcpServers": {
    "rbdc-mcp": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "sqlite://./database.db"]
    }
  }
}
```

**Database Examples:**

<details>
<summary><strong>Different Database Examples</strong></summary>

```json
{
  "mcpServers": {
    "rbdc-mcp-sqlite": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "sqlite://./database.db"]
    },
    "rbdc-mcp-mysql": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "mysql://user:password@localhost:3306/database"]
    },
    "rbdc-mcp-postgres": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "postgres://user:password@localhost:5432/database"]
    },
    "rbdc-mcp-mssql": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "mssql://user:password@localhost:1433/database"]
    },
    "rbdc-mcp-duckdb": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "duckdb://path/to/database.duckdb"]
    },
    "rbdc-mcp-turso": {
      "command": "rbdc-mcp",
      "args": ["--database-url", "turso://database-url?token=your-token"]
    }
  }
}
```
</details>

<details>
<summary><strong>Windows Full Path (if not in PATH)</strong></summary>

```json
{
  "mcpServers": {
    "rbdc-mcp": {
      "command": "C:\\tools\\rbdc-mcp.exe",
      "args": ["--database-url", "sqlite://C:\\path\\to\\database.db"]
    }
  }
}
```
</details>

**Restart:** After saving, restart Claude Desktop to load the MCP server.

**Test:** In Claude Desktop, try asking:
- "Show me the database connection status"
- "What tables are in my database?"

### Codex

**Configuration File Location:**
- **Global**: `~/.codex/mcp.toml`
- **Project-level**: `.codex/mcp.toml` (place in your project root)

**Basic Configuration (`.codex/mcp.toml` or `~/.codex/mcp.toml`):**

```toml
[mcp_servers.rbdc-mcp]
command = "rbdc-mcp"
args = ["--database-url", "sqlite://./database.db"]
type = "stdio"
enabled = true
```

**Database Examples:**

<details>
<summary><strong>Different Database Examples</strong></summary>

```toml
# Just change --database-url to your actual database URL
[mcp_servers.rbdc-mcp]
command = "rbdc-mcp"
args = ["--database-url", "sqlite://./database.db"]
type = "stdio"
enabled = true
```
</details>

**Restart:** After saving the config file, restart Codex to load the MCP server. If Codex is already running, run `codex reconnect` to force a reload.

**Test:** In Codex chat, try asking:
- "Show me the database connection status"
- "What tables are in my database?"

## 📊 Usage Examples

### Natural Language Database Operations

- **Query Data**: "Show me all users in the database"
- **Modify Data**: "Add a new user named John with email john@example.com"
- **Get Status**: "What's the database connection status?"
- **Schema Info**: "What tables exist in my database?"

## 🗄️ Database Support

| Database | Connection URL Format |
|----------|----------------------|
| **SQLite** | `sqlite://path/to/database.db` |
| **MySQL** | `mysql://user:password@host:port/database` |
| **PostgreSQL** | `postgres://user:password@host:port/database` |
| **MSSQL** | `mssql://user:password@host:port/database` |
| **DuckDB** | `duckdb://path/to/database.duckdb` |
| **Turso** | `turso://database-url?token=your-token` |

## ⚙️ Configuration Options

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--database-url, -d` | Database connection URL | *Required* |
| `--max-connections` | Maximum connection pool size | `1` |
| `--timeout` | Connection timeout (seconds) | `30` |
| `--log-level` | Log level (error/warn/info/debug) | `info` |
| `--read-only` | Disable sql_exec and enforce read-only SQL validation | `false` |

## 🛠️ Available Tools

- **`sql_query`**: Execute single read-only SQL queries safely
- **`sql_exec`**: Execute INSERT/UPDATE/DELETE operations when the server is not in read-only mode
- **`db_status`**: Check connection pool status

## Read-Only Mode

`--read-only` disables the `sql_exec` tool, preventing any data modifications. Additionally, `sql_query` validates submitted SQL and rejects statements containing write keywords (INSERT, UPDATE, DELETE, etc.) or multi-statement input.

## 📸 Screenshots

**Step 1: Configuration**
![Configuration](./step1.png)

**Step 2: Usage in Claude**
![Usage](./step2.png)

## License

Apache-2.0 

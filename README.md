# RBDC MCP Server

A database server based on [Model Context Protocol (MCP)](https://modelcontextprotocol.io), supporting SQLite, MySQL, PostgreSQL, and MSSQL databases.

**🇨🇳 中文文档 / Chinese Documentation**: [README_cn.md](./README_cn.md)

## Advantages

- **Multiple Database Support**: Seamlessly work with SQLite, MySQL, PostgreSQL, and MSSQL using a unified interface
- **AI Integration**: Native integration with Claude AI through the Model Context Protocol
- **Zero Configuration**: Automatic management of database connections and resources
- **Security**: Controlled access to your database through AI-driven natural language queries
- **Simplicity**: Use natural language to query and modify your database without writing SQL

## Installation

### 🚀 Method 1: Download Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/rbatis/rbdc-mcp/releases):

| Platform | Download |
|----------|----------|
| **Windows (x64)** | `rbdc-mcp-windows-x86_64.exe` |
| **macOS (Intel)** | `rbdc-mcp-macos-x86_64` |
| **macOS (Apple Silicon)** | `rbdc-mcp-macos-aarch64` |
| **Linux (x64)** | `rbdc-mcp-linux-x86_64` |

**Installation Steps:**
1. Download the appropriate binary for your platform
2. **Unix/macOS**: Make it executable: `chmod +x rbdc-mcp-*`
3. **Optional**: Move to PATH: `mv rbdc-mcp-* /usr/local/bin/rbdc-mcp`
4. Test: `./rbdc-mcp --help`

### 🛠️ Method 2: Install via Cargo

**Prerequisites:** Install [Rust](https://rustup.rs/) first.

```bash
cargo install --git https://github.com/rbatis/rbdc-mcp.git
```

### 🔧 Method 3: Build from Source

```bash
git clone https://github.com/rbatis/rbdc-mcp.git
cd rbdc-mcp
cargo build --release
# Executable: target/release/rbdc-mcp
```

## 🔧 Quick Setup

### Step 1: Configure Claude Desktop

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

**Platform-Specific Examples:**

<details>
<summary><strong>Windows (Downloaded Binary)</strong></summary>

```json
{
  "mcpServers": {
    "rbdc-mcp": {
      "command": "C:\\path\\to\\rbdc-mcp-windows-x86_64.exe",
      "args": ["--database-url", "sqlite://C:\\path\\to\\database.db"]
    }
  }
}
```
</details>

<details>
<summary><strong>macOS/Linux (Downloaded Binary)</strong></summary>

```json
{
  "mcpServers": {
    "rbdc-mcp": {
      "command": "/usr/local/bin/rbdc-mcp",
      "args": ["--database-url", "sqlite:///path/to/database.db"]
    }
  }
}
```
</details>

<details>
<summary><strong>Cargo Installation</strong></summary>

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
</details>

### Step 2: Restart Claude Desktop

After saving the configuration, restart Claude Desktop to load the MCP server.

### Step 3: Test the Connection

In Claude Desktop, try asking:
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

## ⚙️ Configuration Options

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--database-url, -d` | Database connection URL | *Required* |
| `--max-connections` | Maximum connection pool size | `1` |
| `--timeout` | Connection timeout (seconds) | `30` |
| `--log-level` | Log level (error/warn/info/debug) | `info` |

## 🛠️ Available Tools

- **`sql_query`**: Execute SELECT queries safely
- **`sql_exec`**: Execute INSERT/UPDATE/DELETE operations
- **`db_status`**: Check connection pool status

## 📸 Screenshots

**Step 1: Configuration**
![Configuration](./step1.png)

**Step 2: Usage in Claude**
![Usage](./step2.png)

## License

Apache-2.0 
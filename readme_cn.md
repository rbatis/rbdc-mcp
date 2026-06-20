# RBDC MCP Server

基于 [Model Context Protocol (MCP)](https://modelcontextprotocol.io) 的数据库服务器，支持 SQLite、MySQL、PostgreSQL、MSSQL、DuckDB、Turso 六种数据库。

**🇺🇸 English Documentation**: [readme.md](./readme.md)

**🇨🇳 中文文档 / Chinese Documentation**: [readme_cn.md](./readme_cn.md)

## 优势

- **多数据库支持**: 通过统一接口无缝使用 SQLite、MySQL、PostgreSQL、MSSQL、DuckDB、Turso
- **AI 集成**: 通过模型上下文协议 (MCP) 与 Claude AI 原生集成
- **零配置**: 自动管理数据库连接和资源
- **安全性**: 通过 AI 驱动的自然语言查询控制对数据库的访问
- **简单易用**: 使用自然语言查询和修改数据库，无需编写 SQL

## 安装

### 🚀 方式一：通过 Cargo 安装（推荐）

**前置要求：** 先安装 [Rust](https://rustup.rs/)。

根据你的需求选择安装命令：

```bash
# 全部驱动（默认，构建约 10-15 分钟）
cargo install --git https://github.com/rbatis/rbdc-mcp.git

# 最小安装：仅 SQLite（构建最快，约 2-3 分钟）
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features sqlite

# 仅 MySQL：
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features mysql

# 多个驱动：
cargo install --git https://github.com/rbatis/rbdc-mcp.git --no-default-features --features "mysql postgres"
```

**💡 构建加速：** 如果你只用一种数据库（如 SQLite），加上 `--no-default-features --features sqlite` 可跳过无关驱动的编译，从 ~15 分钟降至 ~2 分钟。

#### 可用 Features

| Feature | 驱动 | 说明 |
|---------|------|------|
| `sqlite` | `rbdc-sqlite` | SQLite 支持 |
| `mysql` | `rbdc-mysql` | MySQL 支持 |
| `postgres` | `rbdc-pg` | PostgreSQL 支持 |
| `mssql` | `rbdc-mssql` | MSSQL/SQL Server 支持 |
| `duckdb` | `rbdc-duckdb` | DuckDB 支持 |
| `turso` | `rbdc-turso` | Turso/libsql 支持 |
| `full` | *(以上全部)* | 启用所有数据库驱动 |

### 📦 方式二：下载预编译二进制文件

从 [GitHub Releases](https://github.com/rbatis/rbdc-mcp/releases) 下载适合你平台的最新版本：

| 平台 | 下载文件 |
|------|----------|
| **Windows (x64)** | `rbdc-mcp-windows-x86_64.exe` |
| **macOS (Intel)** | `rbdc-mcp-macos-x86_64` |
| **macOS (Apple Silicon)** | `rbdc-mcp-macos-aarch64` |
| **Linux (x64)** | `rbdc-mcp-linux-x86_64` |

下载后，将文件重命名为 `rbdc-mcp`（Windows 下为 `rbdc-mcp.exe`），并添加到系统 PATH 环境变量中即可。

## 🔧 Agent Client 配置

将 rbdc-mcp 添加到你的 MCP 兼容客户端中即可使用。

### Claude Desktop

**配置文件位置：**
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

**基础配置：**

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

**数据库示例：**

<details>
<summary><strong>不同数据库示例</strong></summary>

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
<summary><strong>Windows 完整路径（如果未添加到PATH）</strong></summary>

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

**重启：** 保存配置后，重启 Claude Desktop 以加载 MCP 服务器。

**测试：** 在 Claude Desktop 中尝试询问：
- "显示数据库连接状态"
- "我的数据库中有哪些表？"

### Codex

**配置文件位置：**
- **全局配置**: `~/.codex/mcp.toml`
- **项目级配置**: `.codex/mcp.toml`（放在项目根目录）

**基础配置（`.codex/mcp.toml` 或 `~/.codex/mcp.toml`）：**

```toml
[mcp_servers.rbdc-mcp]
command = "rbdc-mcp"
args = ["--database-url", "sqlite://./database.db"]
type = "stdio"
enabled = true
```

**数据库示例：**

<details>
<summary><strong>不同数据库示例</strong></summary>

```toml
# 只需修改 --database-url 为你的实际数据库连接即可
[mcp_servers.rbdc-mcp]
command = "rbdc-mcp"
args = ["--database-url", "sqlite://./database.db"]
type = "stdio"
enabled = true
```
</details>

**重启：** 保存配置文件后，重启 Codex 以加载 MCP 服务器。如果 Codex 已在运行，可执行 `codex reconnect` 强制重新加载。

**测试：** 在 Codex 聊天中尝试询问：
- "显示数据库连接状态"
- "我的数据库中有哪些表？"

## 📊 使用示例

### 自然语言数据库操作

- **查询数据**: "显示数据库中的所有用户"
- **修改数据**: "添加一个名为张三、邮箱为zhangsan@example.com的新用户"
- **获取状态**: "数据库连接状态如何？"
- **架构信息**: "我的数据库中有哪些表？"
- **多库协作**: "帮我连上 MySQL 订单库，对比它和本地 SQLite 缓存的记录数"

## 🗄️ 数据库支持

| 数据库 | 连接URL格式 |
|--------|-------------|
| **SQLite** | `sqlite://path/to/database.db` |
| **MySQL** | `mysql://user:password@host:port/database` |
| **PostgreSQL** | `postgres://user:password@host:port/database` |
| **MSSQL** | `mssql://user:password@host:port/database` |
| **DuckDB** | `duckdb://path/to/database.duckdb` |
| **Turso** | `turso://database-url?token=your-token` |

## ⚙️ 配置选项

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--database-url, -d` | 数据库连接URL | *必需* |
| `--max-connections` | 最大连接池大小 | `1` |
| `--timeout` | 连接超时时间（秒） | `30` |
| `--log-level` | 日志级别（error/warn/info/debug） | `info` |
| `--read-only` | 禁用 sql_exec 并启用 SQL 只读校验 | `false` |

## 🛠️ 可用工具

- **`sql_query`**: 安全执行单条只读 SQL 查询
- **`sql_exec`**: 在非只读模式下执行 INSERT/UPDATE/DELETE 操作
- **`db_status`**: 检查连接池状态
- **`sql_query`**: 执行单条只读 SQL 查询。可选传入 `alias` 切到非默认库；省略时使用 `default`。
- **`sql_exec`**: 在非只读模式下执行 INSERT/UPDATE/DELETE 操作。可选传入 `alias` 切到非默认库。
- **`db_status`**: 查看某库的连接池状态。可选传入 `alias`。
- **`test_connection`**: 测试某库的连通性。可选传入 `alias`。
- **`list_databases`**: 列出所有已注册的数据库别名、URL 和识别出的类型。
- **`add_database`**: 运行时注册一个新的数据库连接（指定 `alias` 与 `url`）并建池。支持：`sqlite://`、`mysql://`、`pg://` / `postgres://`、`mssql://` / `sqlserver://`、`duckdb://`、`turso://` / `libsql://`。
- **`remove_database`**: 注销一个已通过 `add_database` 注册的别名；`default` 为保留别名，不可删除。

## 🔌 动态多库

`rbdc-mcp` 是**单进程多库**的 MCP 服务器。通过 `--database-url` 传入的数据库在启动时注册为 `default` 别名；AI 可在对话中通过 MCP 工具继续注册更多数据库，并按 `alias` 路由到任意一个。

**AI 的典型调用流程：**

1. `list_databases` — 查看当前已注册的所有别名。
2. `add_database(alias="orders_mysql", url="mysql://user:pass@host/orders")` — 注册一个新库并建池。
3. `sql_query({ alias: "orders_mysql", sql: "SELECT COUNT(*) FROM orders" })` — 在该库上执行查询。省略 `alias` 时落到 `default`。
4. `remove_database(alias="orders_mysql")` — 用完后拆掉该池并注销别名。

**为什么这样设计**

- 一个 MCP 进程承载多个数据库 —— 不再需要为每个库各起一个 `rbdc-mcp-mysql` / `rbdc-mcp-postgres` 进程。
- 所有别名都可被 `list_databases` 枚举，AI 可在每次调用前动态选目标。
- `default` 别名对应 CLI 启动时传入的 URL，不可删除，保证启动库始终可达。
- 每个别名拥有独立连接池，不同别名之间的并发查询互不阻塞。

**可直接对 AI 说的话**

> "帮我连上 MySQL 订单库 `mysql://root:pwd@10.0.0.5/orders`，按月统计营收。"

AI 会自动调用 `add_database(...)` 注册库，再用 `sql_query(...)` 在该别名上查询，全程不需要重启 MCP 服务器。

## 只读模式

`--read-only` 会禁用 `sql_exec` 工具，阻止任何数据修改操作。同时 `sql_query` 会校验提交的 SQL，拒绝包含写关键字（INSERT、UPDATE、DELETE 等）或多条语句的输入。

## 📸 截图

**步骤 1: 配置**
![配置](./step1.png)

**步骤 2: 在Claude中使用**
![使用](./step2.png)

## 许可证

Apache-2.0 
- **单服务多库**: AI 可通过 `add_database` 工具在运行时动态注册更多数据库连接，并按 `alias` 路由到任意已注册的库 —— 无需额外启动 MCP 进程

# SQLS Extension for Zed

SQL Language Server integration for Zed IDE with auto-completion, syntax checking, and multi-database support.

## Quick Start

Add to your Zed `settings.json`:

```json
{
  "lsp": {
    "sqls": {
      "initialization_options": {
        "connectionConfig": {
          "driver": "sqlite3",
          "dataSourceName": "./database.db"
        }
      }
    }
  }
}
```

## Configuration

### Initialization Options (Startup Only)

**`connectionConfig`** - Primary database connection (required)

```json
"initialization_options": {
  "connectionConfig": {
    "driver": "postgres",
    "user": "user",
    "passwd": "password",
    "host": "localhost",
    "port": 5432,
    "dbName": "mydb"
  }
}
```

### Settings (Runtime)

```json
"settings": {
  "sqls": {
    "lowercaseKeywords": false,
    "connections": [
      { "alias": "dev", "driver": "postgres", ... },
      { "alias": "prod", "driver": "postgres", ... }
    ]
  }
}
```

## Supported Drivers

PostgreSQL, MySQL, SQLite3, MSSQL, H2, Vertica

## Troubleshooting

- **No suggestions?** Restart: `Ctrl+Shift+P` → `editor: restart language server`
- **SQLite?** Use absolute paths
- **Multiple DBs?** Add them to `connections` and switch with code actions

---

*Work In Progress*

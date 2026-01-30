# SQLS Extension for Zed

This extension integrates [sqls](https://github.com/sqls-server/sqls), a SQL Language Server, into the Zed IDE. It provides auto-completion, syntax checking, and column type information for various database drivers.

## Features

- **Automatic Installation**: Downloads the `sqls` binary for your platform automatically.
- **Auto-completion**: Suggestions for tables, columns, and SQL keywords.
- **Rich Labels**: Display column types in the completion menu.
- **Flexible Configuration**: Supports both local and global configuration files.

## How to use

To get started, create a configuration file in your project root. The extension looks for `.sqls/config.yml` or `config.yml`.

### Example Configuration (`.sqls/config.yml`)

#### PostgreSQL
```yml
connections:
  - driver: postgres
    user: your_user
    passwd: your_password
    host: 127.0.0.1
    port: 5432
    dbName: your_db
    params:
      sslmode: disable
```

#### SQLite
```yml
connections:
  - driver: sqlite3
    dataSourceName: /path/to/your/database.db
```

## Troubleshooting

1. **Connection Error**: Check the `sqls.log` file created in your project root for detailed connection errors.
2. **Missing Suggestions**: Ensure your `config.yml` uses absolute paths for SQLite or correct credentials for Postgres/MySQL.
3. **Restart Server**: If you change the database schema, restart the language server in Zed (`ctrl-shift-p` -> `editor: restart language server`).

---
*Note: This extension is a Work In Progress. Feel free to open an issue for suggestions or bug reports.

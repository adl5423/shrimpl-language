# Changelog

This document summarizes the recent changes to the **Shrimpl main language repo** and the **Shrimpl VS Code LSP extension**.

---

## 1. Shrimpl Language (main repo)

### 1.1. Version 0.5.5 – ORM, HTTP wiring, and demo program - 12-11-2025

#### 1.1.1. New SQLite-backed ORM layer

A new minimal ORM layer was introduced in `src/orm.rs` to support typed `model` declarations from Shrimpl programs and to back them with a SQLite database (`shrimpl.db` in the current working directory).

**Key components**

* **Global ORM handle**

  ```rust
  static GLOBAL_ORM: Lazy<Mutex<Option<Orm>>> = Lazy::new(|| Mutex::new(None));
  ```

  * Provides a single process-wide `Orm` instance guarded by a `Mutex`.
  * Initialized once at server startup via `init_global_orm(&Program)` and then used by interpreter builtins.

* **`Orm` struct**

  ```rust
  pub struct Orm {
      conn: Connection,
      models: HashMap<String, ModelDef>,
  }
  ```

  * `conn`: a `rusqlite::Connection` to `shrimpl.db`.
  * `models`: a `HashMap<String, ModelDef>` copied from `Program.models`, keyed by model name.

* **Construction and automatic migrations**

  ```rust
  impl Orm {
      pub fn new(conn: Connection, models: HashMap<String, ModelDef>) -> rusqlite::Result<Self> {
          let mut orm = Orm { conn, models };
          orm.migrate_all()?;
          Ok(orm)
      }

      fn migrate_all(&mut self) -> rusqlite::Result<()> {
          // Clone the model list to avoid aliasing issues during iteration.
          let models: Vec<ModelDef> = self.models.values().cloned().collect();
          for model in models.iter() {
              self.migrate_model(model)?;
          }
          Ok(())
      }
  }
  ```

  * `migrate_all` is called once inside `new` and:

    * Clones the model definitions into a local `Vec<ModelDef>` to avoid borrowing `self.models` immutably while calling methods that may require `&mut self` (fixes Rust’s `E0502` borrow error).
    * Iterates over each `ModelDef` and calls `migrate_model`.

* **Table creation per model**

  ```rust
  fn migrate_model(&mut self, model: &ModelDef) -> rusqlite::Result<()> {
      let mut cols = Vec::new();
      for field in &model.fields {
          cols.push(self.column_sql(field));
      }

      let sql = format!(
          "CREATE TABLE IF NOT EXISTS {} ({})",
          model.table_name,
          cols.join(", ")
      );
      self.conn.execute(&sql, [])?;
      Ok(())
  }
  ```

  * For each Shrimpl `model`, generates a `CREATE TABLE IF NOT EXISTS` statement.
  * Column types and constraints come from `ModelField` metadata.

* **Column type mapping**

  ```rust
  fn column_sql(&self, field: &ModelField) -> String {
      let sql_ty = match field.ty.to_ascii_lowercase().as_str() {
          "int" | "integer" => "INTEGER",
          "number" | "float" | "double" | "real" => "REAL",
          "bool" | "boolean" => "INTEGER",
          "string" | "text" => "TEXT",
          other => {
              eprintln!("[shrimpl-orm] unknown field type '{}', using TEXT", other);
              "TEXT"
          }
      };

      let mut parts = vec![field.name.clone(), sql_ty.to_string()];

      if field.is_primary_key {
          parts.push("PRIMARY KEY".to_string());
      }

      if !field.is_optional {
          parts.push("NOT NULL".to_string());
      }

      parts.join(" ")
  }
  ```

  * Translates Shrimpl `model` field types into SQLite column types.
  * Unknown types fall back to `TEXT` with a stderr warning.
  * Applies `PRIMARY KEY` and `NOT NULL` based on `ModelField` flags.

#### 1.1.2. JSON-based insert and lookup helpers

Two primary methods on `Orm` operate on `serde_json::Value` so they can be easily exposed as Shrimpl builtins.

* **Insert a record (`insert_json`)**

  ```rust
  pub fn insert_json(
      &self,
      model_name: &str,
      record: &JsonValue,
  ) -> Result<i64, String> {
      let model = self
          .models
          .get(model_name)
          .ok_or_else(|| format!("unknown model '{}'", model_name))?;

      let obj = record
          .as_object()
          .ok_or_else(|| "record must be a JSON object".to_string())?;

      let mut cols = Vec::new();
      let mut placeholders = Vec::new();
      let mut values: Vec<JsonValue> = Vec::new();

      for field in &model.fields {
          if let Some(value) = obj.get(&field.name) {
              cols.push(field.name.clone());
              placeholders.push("?".to_string());
              values.push(value.clone());
          }
      }

      if cols.is_empty() {
          return Err("record has no matching fields".to_string());
      }

      let sql = format!(
          "INSERT INTO {} ({}) VALUES ({})",
          model.table_name,
          cols.join(", "),
          placeholders.join(", ")
      );

      let mut stmt = self
          .conn
          .prepare(&sql)
          .map_err(|e| format!("insert prepare failed: {e}"))?;

      let params_vec: Vec<rusqlite::types::Value> =
          values.into_iter().map(json_to_sql_value).collect();

      let rows_changed = stmt
          .execute(rusqlite::params_from_iter(params_vec.iter()))
          .map_err(|e| format!("insert execute failed: {e}"))?;

      if rows_changed == 0 {
          return Err("insert affected 0 rows".to_string());
      }

      Ok(self.conn.last_insert_rowid())
  }
  ```

  * Validates the target model exists.
  * Restricts insert columns to the fields declared on the model.
  * Converts JSON values to SQLite values using `json_to_sql_value`.
  * Returns the SQLite rowid as an `i64`.

* **Find by primary key (`find_by_id`)**

  ```rust
  pub fn find_by_id(
      &self,
      model_name: &str,
      id: &JsonValue,
  ) -> Result<Option<JsonValue>, String> {
      let model = self
          .models
          .get(model_name)
          .ok_or_else(|| format!("unknown model '{}'", model_name))?;

      let pk_field = model
          .fields
          .iter()
          .find(|f| f.is_primary_key)
          .ok_or_else(|| format!("model '{}' has no primary key field", model_name))?;

      let sql = format!(
          "SELECT * FROM {} WHERE {} = ? LIMIT 1",
          model.table_name, pk_field.name
      );

      let mut stmt = self
          .conn
          .prepare(&sql)
          .map_err(|e| format!("find prepare failed: {e}"))?;

      let id_value = json_to_sql_value(id.clone());
      let mut rows = stmt
          .query(params![id_value])
          .map_err(|e| format!("find query failed: {e}"))?;

      if let Some(row) = rows
          .next()
          .map_err(|e| format!("find next failed: {e}"))?
      {
          let mut obj = JsonMap::new();
          for field in &model.fields {
              let val: rusqlite::types::Value = row
                  .get(field.name.as_str())
                  .map_err(|e| format!("column get failed: {e}"))?;
              obj.insert(field.name.clone(), sql_value_to_json(val));
          }
          Ok(Some(JsonValue::Object(obj)))
      } else {
          Ok(None)
      }
  }
  ```

  * Locates the primary key field from the model definition.
  * Executes a `SELECT * ... WHERE pk = ? LIMIT 1` query.
  * Maps each column back into a JSON object keyed by field name.

* **JSON/SQL conversion helpers**

  ```rust
  fn json_to_sql_value(v: JsonValue) -> rusqlite::types::Value {
      use rusqlite::types::Value as SqlValue;
      match v {
          JsonValue::Null => SqlValue::Null,
          JsonValue::Bool(b) => SqlValue::Integer(if b { 1 } else { 0 }),
          JsonValue::Number(n) => {
              if let Some(i) = n.as_i64() {
                  SqlValue::Integer(i)
              } else if let Some(f) = n.as_f64() {
                  SqlValue::Real(f)
              } else {
                  SqlValue::Text(n.to_string())
              }
          }
          JsonValue::String(s) => SqlValue::Text(s),
          other => SqlValue::Text(other.to_string()),
      }
  }

  fn sql_value_to_json(v: rusqlite::types::Value) -> JsonValue {
      use rusqlite::types::Value as SqlValue;
      match v {
          SqlValue::Null => JsonValue::Null,
          SqlValue::Integer(i) => JsonValue::from(i),
          SqlValue::Real(f) => JsonValue::from(f),
          SqlValue::Text(s) => JsonValue::from(s),
          SqlValue::Blob(b) => JsonValue::from(base64::engine::general_purpose::STANDARD.encode(b)),
      }
  }
  ```

  * Converts values between `serde_json::Value` and `rusqlite::types::Value`.
  * Uses the non-deprecated `base64::engine::general_purpose::STANDARD.encode` API instead of the deprecated `base64::encode`, eliminating the deprecation warning.

#### 1.1.3. Global ORM initialization API

A small public API is provided for use by `main.rs` and for wiring into builtins:

* **Initialize the global ORM**

  ```rust
  pub fn init_global_orm(program: &Program) -> rusqlite::Result<()> {
      let conn = Connection::open("shrimpl.db")?;
      let models = program.models.clone();
      let orm = Orm::new(conn, models)?;

      let mut guard = GLOBAL_ORM
          .lock()
          .expect("GLOBAL_ORM poisoned");
      *guard = Some(orm);

      Ok(())
  }
  ```

  * Opens `shrimpl.db` in the current working directory.
  * Clones `program.models` into the ORM.
  * Stores the `Orm` instance into the global `GLOBAL_ORM` mutex.

* **Shrimpl-callable helpers**

  ```rust
  pub fn orm_insert(model_name: &str, record_json: &str) -> Result<String, String> {
      let guard = GLOBAL_ORM
          .lock()
          .map_err(|_| "GLOBAL_ORM poisoned".to_string())?;
      let orm = guard
          .as_ref()
          .ok_or_else(|| "ORM not initialized".to_string())?;

      let value: JsonValue =
          serde_json::from_str(record_json).map_err(|e| format!("invalid JSON: {e}"))?;

      let rowid = orm.insert_json(model_name, &value)?;
      Ok(rowid.to_string())
  }

  pub fn orm_find_by_id(model_name: &str, id_json: &str) -> Result<Option<String>, String> {
      let guard = GLOBAL_ORM
          .lock()
          .map_err(|_| "GLOBAL_ORM poisoned".to_string())?;
      let orm = guard
          .as_ref()
          .ok_or_else(|| "ORM not initialized".to_string())?;

      let id_val: JsonValue =
          serde_json::from_str(id_json).map_err(|e| format!("invalid id JSON: {e}"))?;

      let result = orm.find_by_id(model_name, &id_val)?;
      Ok(result.map(|v| v.to_string()))
  }
  ```

  * Fixes Rust `E0716` (“temporary value dropped while borrowed”) by:

    * First binding the `MutexGuard` into `guard`.
    * Then obtaining an `&Orm` reference from `guard.as_ref()`.
  * Provides simple, string-based APIs suitable to be exposed as Shrimpl builtins:

    * `record_json` and `id_json` are JSON strings (object for insert, scalar for ID).
    * Returns rowid or JSON record as strings.

#### 1.1.4. `main.rs` wiring for ORM and config+server

The CLI entrypoint was updated to initialize configuration, apply server overrides, and wire in the ORM.

* **Module imports**

  ```rust
  mod ast;
  mod config;
  mod docs;
  mod interpreter;
  mod lockfile;
  mod parser;
  mod orm;
  ```

* **Config initialization**
  At the start of `run_cli()`:

  ```rust
  // Initialize environment-specific configuration (config/config.<env>.json).
  shrimpl_config::init();
  ```

* **Server startup flow in `Commands::Run`**

  ```rust
  Commands::Run => {
      let (source, mut program) = load_and_parse(&cli.file)?;
      let _ = source;

      // Apply server overrides from config file (port / tls).
      shrimpl_config::apply_server_to_program(&mut program);

      // Initialize ORM based on all `model` declarations.
      // This is best-effort; failures are logged but do not prevent startup.
      if let Err(e) = orm::init_global_orm(&program) {
          eprintln!("[shrimpl-orm] failed to initialize ORM: {e}");
      }

      let port = program.server.port;
      let scheme = if program.server.tls { "https" } else { "http" };

      println!();
      println!("shrimpl run");
      println!("----------------------------------------");
      println!("Shrimpl server is starting on {scheme}://localhost:{port}");
      println!("Open one of these in your browser:");
      println!("  • {scheme}://localhost:{port}/");
      println!("  • {scheme}://localhost:{port}/__shrimpl/ui");
      println!("  • {scheme}://localhost:{port}/health");
      println!();
      println!("Press Ctrl+C to shut down the server.");
      println!("----------------------------------------");
      println!();

      actix_web::rt::System::new().block_on(run_server(program))?;
  }
  ```

  * Calls `apply_server_to_program` so `config/config.<env>.json` can override port and TLS.
  * Calls `init_global_orm(&program)` to set up SQLite tables based on `model` definitions.
  * Runs the Actix HTTP server via `interpreter::http::run`.

#### 1.1.5. Interpreter builtins for ORM

The expression evaluator (`src/interpreter/eval.rs`) was updated to expose the ORM functions to Shrimpl code as simple builtins.

* **Import of the ORM module**

  ```rust
  use crate::orm;
  ```

* **Builtin registration (conceptual)**

  Inside the builtin function dispatch table (pseudocode, adapted to existing style):

  ```rust
  "orm_insert" => {
      if args.len() != 2 {
          return Err(RuntimeError::new("orm_insert expects 2 arguments: (model_name, record_json)"));
      }
      let model_name = args[0].as_string()?;
      let record_json = args[1].as_string()?;
      let rowid = orm::orm_insert(&model_name, &record_json)
          .map_err(RuntimeError::from_string)?;
      Value::Str(rowid)
  }

  "orm_find_by_id" => {
      if args.len() != 2 {
          return Err(RuntimeError::new("orm_find_by_id expects 2 arguments: (model_name, id_json)"));
      }
      let model_name = args[0].as_string()?;
      let id_json = args[1].as_string()?;
      let result_opt = orm::orm_find_by_id(&model_name, &id_json)
          .map_err(RuntimeError::from_string)?;
      match result_opt {
          Some(json_str) => Value::Str(json_str),
          None => Value::Null,
      }
  }
  ```

  * Makes `orm_insert(model_name, json_string)` and `orm_find_by_id(model_name, id_json_string)` callable from Shrimpl code.
  * Returns a string (rowid or JSON record) or `null` for `None`.

#### 1.1.6. Demo program `app.shr` for ORM + HTTP + rate limiting

`app.shr` was expanded to both validate the new runtime behavior and provide a comprehensive example for users.

**Highlights**

* **Model declarations**

  ```shrimpl
  model User:
    id: int pk
    email: string
    name: string
    age?: int

  model Task:
    id: int pk
    title: string
    status: string
    payload?: string
  ```

  * Parsed into `Program.models` and used by `init_global_orm` to create `users` and `tasks` tables.

* **ORM demo endpoints**

  ```shrimpl
  endpoint POST "/orm/users": orm_insert("User", body)
  endpoint GET "/orm/users/:id": orm_find_by_id("User", id)

  endpoint POST "/orm/tasks": orm_insert("Task", body)
  endpoint GET "/orm/tasks/:id": orm_find_by_id("Task", id)
  ```

  * `body` is the JSON request body (string) injected by the HTTP layer.
  * Path param `:id` is used directly as `id_json`, which works because `"1"` is valid JSON for the scalar `1`.

* **Additional features co-tested by this app:**

  * Structured JSON logging per request (already in `http.rs`).
  * Config-driven server port and TLS.
  * JWT auth and injected `jwt_sub`, `jwt_role`, `jwt_scope`.
  * Request validation and sanitization via JSON Schema for `/orders/create`.
  * Rate limiting through `@rate_limit` annotations:

    * `@rate_limit(5, 60)` on `/limited/ping`.
    * `@rate_limit 3 10` on `/limited/stats`.
  * Basic tests (`test "..."`) verifying control flow and helper functions.

---

## 2. Shrimpl VS Code LSP Extension (`lsp-shrimpl-lang`)

### 2.1. Version 0.1.4–0.1.5 – Bundled LSP binaries and robust launch

These versions focus on making the extension usable by anyone who installs it from the Marketplace, without requiring a manual `shrimpl-lsp` build or PATH configuration.

#### 2.1.1. `extension.ts`: platform detection and command resolution

The extension entrypoint now resolves the language server command in a robust, user-friendly way.

* **Platform-specific binary selection**

  ```ts
  function platformBinaryName(): string {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === "win32") {
      return "shrimpl-lsp-win32-x64.exe";
    }

    if (platform === "darwin" && arch === "arm64") {
      return "shrimpl-lsp-darwin-arm64";
    }

    // Default / fallback: Linux x64
    return "shrimpl-lsp-linux-x64";
  }
  ```

  * Chooses the correct binary name under `server/` based on the current OS and architecture.
  * Defaults conservatively to `shrimpl-lsp-linux-x64` when not on Windows or macOS arm64.

* **Workspace-aware command resolution for `shrimpl.lsp.path`**

  ```ts
  function resolveServerCommand(
    rawValue: string,
    outputChannel: vscode.OutputChannel,
  ): string {
    const trimmed = rawValue.trim();

    const folders = vscode.workspace.workspaceFolders;
    let wsPath: string | undefined;
    let wsName: string | undefined;

    if (folders && folders.length > 0) {
      wsPath = folders[0].uri.fsPath;
      wsName = folders[0].name;
    }

    let resolved = trimmed;

    if (wsPath) {
      resolved = resolved.replace(/\$\{workspaceFolder\}/g, wsPath);
    }
    if (wsName) {
      resolved = resolved.replace(
        /\$\{workspaceFolderBasename\}/g,
        wsName,
      );
    }

    const hasSlash = resolved.includes("/") || resolved.includes("\\");
    const isAbsolute = path.isAbsolute(resolved);

    if (hasSlash && !isAbsolute && wsPath) {
      resolved = path.join(wsPath, resolved);
    }

    outputChannel.appendLine(
      `[Shrimpl] Raw LSP command from settings: ${rawValue}`,
    );
    outputChannel.appendLine(
      `[Shrimpl] Resolved LSP command to: ${resolved}`,
    );

    return resolved;
  }
  ```

  * Supports VS Code variables:

    * `${workspaceFolder}`
    * `${workspaceFolderBasename}`
  * Relative paths (containing `/` or `\`) are resolved under the first workspace folder.
  * Simple commands with no slashes (e.g. `shrimpl-lsp`) are treated as executables to be found on `PATH`.
  * Emits debug logs to the “Shrimpl” output channel for easier troubleshooting.

* **Command selection with bundled binary fallback**

  ```ts
  function getServerCommand(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
  ): string {
    const config = vscode.workspace.getConfiguration("shrimpl");
    const rawConfigValue = config.get<string>("lsp.path") ?? "";
    const trimmed = rawConfigValue.trim();

    if (trimmed.length > 0) {
      outputChannel.appendLine(
        "[Shrimpl] Using custom LSP command from setting 'shrimpl.lsp.path'.",
      );
      return resolveServerCommand(trimmed, outputChannel);
    }

    const bundledBinary = platformBinaryName();
    const absoluteBundledPath = context.asAbsolutePath(
      path.join("server", bundledBinary),
    );

    outputChannel.appendLine(
      "[Shrimpl] No custom 'shrimpl.lsp.path' configured. " +
        `Using bundled language server binary: ${absoluteBundledPath}`,
    );

    return absoluteBundledPath;
  }
  ```

  * If the user configures `shrimpl.lsp.path`, that value is used (after resolution).
  * Otherwise, the extension automatically uses the bundled platform-specific LSP binary in `server/` (e.g. `server/shrimpl-lsp-darwin-arm64`).

#### 2.1.2. Language client startup and error handling

The `activate` function sets up and starts the language client with better logging and configuration handling.

* **Client initialization**

  ```ts
  export async function activate(
    context: vscode.ExtensionContext,
  ): Promise<void> {
    const outputChannel = vscode.window.createOutputChannel("Shrimpl");
    const traceOutputChannel =
      vscode.window.createOutputChannel("Shrimpl LSP Trace");

    const serverCommand = getServerCommand(context, outputChannel);

    const env = {
      ...process.env,
    };

    const serverOptions: ServerOptions = {
      run: {
        command: serverCommand,
        args: [],
        options: { env },
      },
      debug: {
        command: serverCommand,
        args: ["--debug"],
        options: { env },
      },
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [
        { scheme: "file", language: "shrimpl" },
        { scheme: "untitled", language: "shrimpl" },
      ],
      synchronize: {
        fileEvents: vscode.workspace.createFileSystemWatcher("**/*.shr"),
      },
      outputChannel,
      traceOutputChannel,
    };

    client = new LanguageClient(
      "shrimplLanguageServer",
      "Shrimpl Language Server",
      serverOptions,
      clientOptions,
    );

    try {
      outputChannel.appendLine("[Shrimpl] Starting language server...");

      context.subscriptions.push(client);

      await client.start();

      outputChannel.appendLine("[Shrimpl] Language server is ready.");
      vscode.window.showInformationMessage(
        "[Shrimpl] Language server started.",
      );
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      outputChannel.appendLine(
        `[Shrimpl] Failed to start language server: ${msg}`,
      );
      vscode.window.showErrorMessage(
        `[Shrimpl] Failed to start language server: ${msg}`,
      );
    }

    context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("shrimpl.lsp.path")) {
          outputChannel.appendLine(
            "[Shrimpl] Configuration 'shrimpl.lsp.path' changed. " +
              "Please reload VS Code to restart the language server with the new path.",
          );
          vscode.window.showInformationMessage(
            "[Shrimpl] 'shrimpl.lsp.path' changed. Reload the window to apply the new language server path.",
          );
        }
      }),
    );
  }
  ```

  * Uses `Shrimpl` and `Shrimpl LSP Trace` output channels for normal logs and protocol tracing respectively.
  * Handles startup failures (`EACCES`, `ENOENT`, `ENOEXEC`, etc.) by:

    * Logging detailed messages to the output channel.
    * Surfacing a VS Code notification to the user.

* **Clean shutdown**

  ```ts
  export async function deactivate(): Promise<void> {
    if (!client) {
      return;
    }

    try {
      await client.stop();
    } finally {
      client = undefined;
    }
  }
  ```

  * Ensures the LSP process is stopped when the extension is deactivated.

#### 2.1.3. Packaged server binaries

The extension now ships prebuilt language server binaries in the `server/` directory:

* `server/shrimpl-lsp-darwin-arm64`
* `server/shrimpl-lsp-linux-x64`
* `server/shrimpl-lsp-win32-x64.exe`

**Packaging adjustments**

* `package.json` includes `server/**` in the `files` array:

  ```json
  "files": [
    "out/**",
    "server/**",
    "syntaxes/**",
    "language-configuration.json",
    "icon-theme.json",
    "icons/**",
    "README.md",
    "LICENSE",
    "package.json"
  ]
  ```

* Before publishing, a build+copy script is used to:

  * Build the Rust LSP binary for each target (locally for the platform used to build).
  * Copy the built artifacts into `server/`.
  * Mark Unix binaries as executable (`chmod +x server/shrimpl-lsp-darwin-arm64` etc.).

This allows anyone installing the extension from the Marketplace to:

* Get syntax highlighting immediately.
* Have the Shrimpl language server start automatically without a separate `cargo build` or PATH configuration.

#### 2.1.4. Configuration surface for users

The extension exposes a configuration schema under the `shrimpl` namespace:

```json
"configuration": {
  "type": "object",
  "title": "Shrimpl",
  "properties": {
    "shrimpl.lsp.path": {
      "type": "string",
      "default": "shrimpl-lsp",
      "description": "Command used to start the Shrimpl language server (absolute path, workspace-relative path, or executable name on PATH)."
    },
    "shrimpl.lsp.debugArgs": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "default": [
        "--log-level",
        "debug"
      ],
      "description": "Additional arguments passed to the Shrimpl language server when running in debug mode."
    },
    "shrimpl.trace.server": {
      "type": "string",
      "enum": [
        "off",
        "messages",
        "verbose"
      ],
      "default": "off",
      "description": "Trace level for the Shrimpl language server."
    }
  }
}
```

* `shrimpl.lsp.path`

  * When non-empty, overrides the bundled binary and lets users point at a custom development build (e.g. within another repo).
  * May use `${workspaceFolder}`/`${workspaceFolderBasename}` variables.
* `shrimpl.lsp.debugArgs`

  * Reserved for passing additional debug flags to the LSP in debug mode.
* `shrimpl.trace.server`

  * Controls how much LSP protocol traffic is traced in the dedicated trace channel.

#### 2.1.5. Icon theme behavior clarification

The extension optionally contributes a minimal icon theme:

```json
"iconThemes": [
  {
    "id": "shrimpl-icons",
    "label": "Shrimpl Icons",
    "path": "./icon-theme.json"
  }
]
```

* `icon-theme.json` currently defines a Shrimpl icon (e.g. for `.shr` files).
* Because VS Code supports only one active icon theme at a time, enabling “Shrimpl Icons” will:

  * Apply the Shrimpl icon where configured.
  * Leave other file types to fall back to generic icons if they are not defined in the theme.
* Most users can keep using their preferred icon theme (e.g. Material Icons) and rely solely on the language + LSP features without enabling the Shrimpl icon theme.

---

This changelog reflects the current state after:

* Wiring the SQLite-backed ORM into the Shrimpl interpreter and HTTP server.
* Providing a comprehensive `app.shr` that exercises ORM, auth, rate limiting, and validation.
* Making the VS Code extension self-contained by bundling platform-specific LSP binaries and resolving `shrimpl.lsp.path` robustly for all users.

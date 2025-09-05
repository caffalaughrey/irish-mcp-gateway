# MCP in Rust (rmcp 0.5) — Happy Path vs. Traps

## Final “Happy Path” (what to do)
All happy path notes below can be observed [in this branch](https://github.com/caffalaughrey/irish-mcp-gateway/tree/0b175a69216d283df2ceaf7f2f2fe9f1fcdc9c43), particularly in the project `Cargo.toml` and `/src/infra
/mcp.rs` files.


### 1) Dependencies & Features
- `rmcp = { version = "0.5", features = [ "server", "macros", "transport-io", "server-side-http", "tower", "transport-streamable-http-server", "transport-streamable-http-server-session", "transport-worker" ] }`
- **Do not** add `schemars` directly. If you ever need it, use `rmcp::schemars` re-export only, but we currently **avoid** schema derivations entirely.

### 2) Canonical imports (copy/paste)
```rust
use std::{future::Future, pin::Pin, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    model::{CallToolResult, Content, JsonObject},
    handler::server::{
        router::Router,
        tool::{Parameters, ToolRouter},
    },
    serve_server,
};

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpService, StreamableHttpServerConfig},
};
```

### 3) Tool handler design
- Implement the handler:  
  `impl ServerHandler for GatewaySvc {}`
- Use the **macro-generated** tool router:  
  `#[rmcp::tool_router] impl GatewaySvc { … }`
- **Arguments**: accept `Parameters<JsonObject>` (no schema derivations; robust & simple).
- **Return**: return **structured JSON** via `rmcp::Json<Value>` **or** `CallToolResult::success(vec![Content::json(...)])`. In this branch we use the **strongest** spec path: **return `rmcp::Json<Value>`**.

**Canonical tool method (what we shipped):**
```rust
#[rmcp::tool(name = "gael.grammar_check",
             description = "Run Gramadóir and return {\"issues\": [...]} exactly as JSON")]
async fn gael_grammar_check(
    &self,
    params: Parameters<JsonObject>,
) -> Result<rmcp::Json<serde_json::Value>, McpError> {
    let text = params
        .0
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("missing required field: text", None))?
        .to_owned();

    let payload = self
        .checker
        .check_as_json(&text)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(rmcp::Json(payload)) // → MCP structuredContent
}
```

> Why: returning `rmcp::Json<Value>` guarantees the transport emits **`structuredContent`** (JSON), not a double-encoded `"text"` blob.

### 4) Stdio transport (MODE=stdio)
- Build a **Service<RoleServer>** via `Router::new(handler).with_tools(tool_router)`.
- Use **Tokio** stdio, not a custom `stdio()` helper.
- Call the re-exported `serve_server(service, (stdin, stdout))`.

```rust
pub async fn serve_stdio(
    factory: impl FnOnce() -> (GatewaySvc, ToolRouter<GatewaySvc>),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (handler, tools) = factory();
    let service = Router::new(handler).with_tools(tools);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    serve_server(service, (stdin, stdout)).await?;
    Ok(())
}
```

### 5) Streamable HTTP transport (SSE + POST at `/mcp`)
- Construct a **StreamableHttpService** whose factory returns a **Service<RoleServer>** (i.e., `Router<GatewaySvc>`), **not** a raw `(handler, ToolRouter)` pair.
- Reuse a single `Arc<LocalSessionManager>` (stateful mode).

```rust
pub fn make_streamable_http_service(
    factory: impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Send + Sync + Clone + 'static,
    session_mgr: Arc<LocalSessionManager>,
) -> StreamableHttpService<Router<GatewaySvc>, LocalSessionManager> {
    let cfg = StreamableHttpServerConfig::default();

    let service_factory = move || {
        let (handler, tools) = factory();
        let service = Router::new(handler).with_tools(tools);
        Ok(service)
    };

    StreamableHttpService::new(service_factory, session_mgr, cfg)
}
```

Headers (stateful): for POSTs (e.g., `tools/list`, `tools/call`) send `Accept: application/json, text/event-stream` and `Content-Type: application/json`. Include `MCP-Session-Id` after initialize.

### 6) Errors
- Use `ErrorData` helpers (`McpError::invalid_params`, `McpError::internal_error`, …).
- Don’t invent your own error type for tool calls.

### 7) Tests (unit)
- Call the tool directly with `Parameters<JsonObject>`.
- For error tests, **don’t** use `unwrap_err()` when Ok type lacks `Debug`; **match** on `Result` and assert `err.code == -32602`.

---

## Unhappy Paths (observed pitfalls → symptoms → fixes)

### A) Returning tuples where a Service is required
- **Symptom:**  
  ```
  the trait bound `(GatewaySvc, ToolRouter<GatewaySvc>): rmcp::Service<RoleServer>` is not satisfied
  ```
- **Cause:** passing `(handler, router)` into `StreamableHttpService::new` or `serve_server`.
- **Fix:** always wrap: `Router::new(handler).with_tools(router)` and pass **that** to the transport.

### B) Using `Content::json(payload)` but getting `"type":"text"`
- **Symptom:** serialized tool result shows:
  ```json
  { "content": [ { "type": "text", "text": "{\"issues\":...}" } ] }
  ```
- **Cause:** using a constructor/path that serializes JSON as text in this build.
- **Fix (chosen):** change the tool method to return `rmcp::Json<Value>` (most spec-compliant and unambiguous).

### C) Wrong module paths (0.4 vs. 0.5 mental model)
- **Symptoms:**
  - `use rmcp::content::Content;` → not found
  - `use rmcp::handler::server::tool::ToolError;` → not found
  - `use rmcp::handler::server::tower::TowerHandler;` → not found
- **Fix:**
  - `Content` and `CallToolResult` are under `rmcp::model`.
  - No public `ToolError`; use `ErrorData` helper ctors.
  - No `TowerHandler`; use `rmcp::handler::server::Router`.

### D) `serve_server` path
- **Symptom:**  
  `cannot find function serve_server in rmcp::transport::io`
- **Fix:** import the crate re-export: `use rmcp::serve_server;`

### E) `Parameters<T>` + schema derivations
- **Symptoms:** `CheckInput: JsonSchema not satisfied` or derive macro errors for `JsonSchema`.
- **Cause:** typed params need schema; multiple `schemars` versions will also explode.
- **Fix (chosen):** **avoid** schema for now; use `Parameters<JsonObject>`. If schema is ever needed, derive with **`rmcp::schemars::JsonSchema`** (not a direct `schemars` dep) and ensure there’s only **one** schemars in the tree.

### F) Using method-style `params.deserialize()`
- **Symptom:** “no method named `deserialize`” on `Parameters<T>`.
- **Cause:** In 0.5, `Parameters<T>` is a **tuple struct**; use `params.0`.
- **Fix:** `let text = params.0.get("text")…`

### G) Error message type inference weirdness
- **Symptom:** `cannot satisfy _: Into<Cow<'static, str>>` on invalid_params call.
- **Cause:** mixing `.into()` with type inference edge-cases.
- **Fix:** pass a `&'static str` directly:  
  `McpError::invalid_params("missing required field: text", None)`

### H) Using `unwrap_err()` when Ok type lacks `Debug`
- **Symptom:** `rmcp::Json<Value> cannot be formatted using {:?}`
- **Fix:** pattern-match `Result`:  
  `let err = match res { Err(e) => e, Ok(_) => panic!(...) };`

---

## Minimal API surface we rely on (to keep future diffs low)

- **Tool macro & router:** `#[rmcp::tool_router]`, `ToolRouter<GatewaySvc>`.
- **Handler trait:** `ServerHandler` (crate root).
- **Typed content:** return `rmcp::Json<Value>` for structured output.
- **Transports:**  
  - stdio: `serve_server(service, (stdin, stdout))`  
  - streamable-http: `StreamableHttpService::new(factory -> Service<RoleServer>, Arc<LocalSessionManager>, StreamableHttpServerConfig::default())`

---

## What to avoid recommending (in future chats)

- Adding a direct `schemars` dependency.
- Returning `(handler, router)` to transports (always produce `Router::new(handler).with_tools(router)`).
- Using `ToolError`, `TowerHandler`, or `content::Content` (wrong modules or removed).
- Emitting JSON as `"type":"text"`; always prefer `rmcp::Json(Value)` for structured content.
- Suggesting method-style `params.deserialize()`; use `params.0` with `JsonObject`.

---

## Quick compliance checklist

- [x] Tool returns **structured JSON** (`rmcp::Json<Value>`) → `structuredContent` on wire.  
- [x] Stdio server uses `serve_server(Router, (stdin, stdout))`.  
- [x] Streamable HTTP server uses `StreamableHttpService` fed by a **`Service<RoleServer>`** factory and a shared `Arc<LocalSessionManager>`.  
- [x] No direct `schemars` dep; no schema derivations required.  
- [x] Errors via `ErrorData` helpers.  
- [x] `Parameters<JsonObject>` for inputs; robust and schema-free.

### Handshake & headers (stateful HTTP)
- [x] POST initialize (no session id) → capture `MCP-Session-Id` from response header.
- [x] POST `notifications/initialized` with that session id → expect 202.
- [x] Subsequent POSTs include `MCP-Session-Id`, `Accept: application/json, text/event-stream`, `Content-Type: application/json`.

---

If we stick to this happy path, we stay out of rmcp 0.5 foot-guns and keep diffs tiny as the SDK evolves.

# Integration Test Findings: Streamable HTTP Service (rmcp 0.5)

This document captures the tested, spec-aligned “happy path” for rmcp 0.5 `StreamableHttpService` with stateful HTTP and SSE. It also records pitfalls we hit and how to avoid them.

## Happy Path for Integration Tests

### 1. Dependencies

Ensure `http-body-util = "0.1.3"` is included in `Cargo.toml` for efficient HTTP body manipulation. For tests, add tracing deps: `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }`.

### 2. StreamableHttpService Initialization

The `StreamableHttpService` should be initialized with a shared `Arc<LocalSessionManager>` to maintain session state across requests within a test. The `make_streamable_http_service` function in `src/infra/mcp.rs` should accept this shared manager.

```rust
pub fn make_streamable_http_service(
    factory: impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Send + Sync + Clone + 'static,
    session_mgr: Arc<LocalSessionManager>,
) -> StreamableHttpService<GatewayRouter, LocalSessionManager> {
    // ...
}
```

### 3. Handshake order (required)

1) POST /mcp initialize
- Headers: `Accept: application/json, text/event-stream`, `Content-Type: application/json`.
- No `MCP-Session-Id` on first call; server returns SSE body with the initialize result and a `MCP-Session-Id` response header.

2) POST /mcp notifications/initialized
- Headers: `Accept: application/json, text/event-stream`, `Content-Type: application/json`, include `MCP-Session-Id` from step 1.
- Body: `{ "jsonrpc":"2.0", "method":"notifications/initialized", "params":{} }`.
- Response: `202 Accepted`.

Without step (2), subsequent requests fail with “expect initialized notification”.

### 4. Request/response model in stateful mode

- For a valid session, a POST request (e.g., `tools/list`, `tools/call`) with headers:
  - `Accept: application/json, text/event-stream`
  - `Content-Type: application/json`
  - `MCP-Session-Id: <session>`
  returns an SSE body for that POST (per-request SSE channel). The JSON-RPC response appears as an SSE `data: { ... }` line in the POST body.

- GET /mcp can be used to open a standalone SSE stream for server-initiated messages, but is not required for per-request responses.

### 5. Test-side verification patterns

- initialize: check status is success; read body bytes; strip optional `data: ` prefix to parse JSON.
- notifications/initialized: assert `202 Accepted`.
- tools/list and tools/call: collect the POST body, scan lines for `data: `, parse the JSON-RPC response from that line.
- Be aware of keep-alives (`:\n\n`). When scanning lines, look for `data: ` entries specifically.

### 6. Timeouts

Employ `tokio::time::timeout` to prevent tests from hanging indefinitely. Ensure timeouts are sufficiently long to accommodate the server's `sse_keep_alive` duration (e.g., 15 seconds by default in `rmcp`).

## Implementation Tweaks and Pitfalls

*   **Initial `data: ` prefix expectation**: Debugging revealed that the first `Frame::Data` from the SSE stream might be a keep-alive ping (`:\n\n`), not necessarily a `data: ` prefixed JSON message. This led to incorrect assumptions about the initial stream content.
*   **`StreamableHttpService` State Management**: Understanding that `StreamableHttpService` is `Clone` and its `LocalSessionManager` is shared via `Arc` was key to correctly maintaining session state across `oneshot` calls within a single test.
*   **Reference File (`tower.rs`) vs. Compiled Crate**: Debugging by modifying local reference files like `tower.rs` was ineffective as the actual compiled `rmcp` crate was in use. Debugging efforts needed to focus on observing the existing behavior of the crate.
*   **Higher-Level Client Parsing**: For application code, prefer client libs to parse SSE. For tests, minimal `data: ` line parsing is sufficient and robust.
*   **Tools output**: `tools/call` returns JSON-RPC response with `result.structuredContent` holding the plain JSON payload (e.g., `issues`). Adjust assertions accordingly.

## Appendix: Relevant Links

*   **`rmcp` Crate (v0.5.0) Documentation**: [https://docs.rs/crate/rmcp/0.5.0](https://docs.rs/crate/rmcp/0.5.0)
*   **MCP Specification**: [https://modelcontextprotocol.io/quickstart/server](https://modelcontextprotocol.io/quickstart/server)
*   **`rmcp-agent` GitHub Repository**: [https://github.com/ZBcheng/rmcp-agent](https://github.com/ZBcheng/rmcp-agent)
*   **`streaming_with_rmcp_tools.rs` Example**: [https://github.com/ZBcheng/rmcp-agent/blob/main/examples/streaming_with_rmcp_tools.rs](https://github.com/ZBcheng/rmcp-agent/blob/main/examples/streaming_with_rmcp_tools.rs)

use std::sync::Arc;

use axum::{routing::any_service, Router};
use hyper::{header, Request};
use serde_json::{json, Value};
use tower::ServiceExt; // for .oneshot

use irish_mcp_gateway::infra::mcp;

const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

// Minimal fake GrammarCheck: deterministic "issues"
struct FakeChecker;
#[async_trait::async_trait]
impl irish_mcp_gateway::infra::mcp::GrammarCheck for FakeChecker {
    async fn check_as_json(
        &self,
        _text: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(json!({"issues":[{"code":"TEST","message":"ok","start":0,"end":1,"suggestions":[]}]}))
    }
}

fn app() -> Router {
    // mount streamable MCP service only (no state)
    let checker = Arc::new(FakeChecker) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let svc = mcp::make_streamable_http_service(move || mcp::factory_with_checker(checker.clone()));
    Router::new().route_service("/mcp", any_service(svc))
}

#[tokio::test]
async fn initialize_returns_server_info_and_capabilities() {
    let app = app();
    let frame = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"clientInfo":{"name":"it","version":"0.0"},"capabilities":{}}
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success(),
        "status={} body={}",
        res.status(),
        String::from_utf8_lossy(&axum::body::to_bytes(res.into_body(), 1<<20).await.unwrap())
    );

    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["result"]["serverInfo"].is_object());
    assert!(v["result"]["capabilities"].is_object());
}

#[tokio::test]
async fn tools_list_includes_grammar_check() {
    let app = app();
    let frame = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let tools = v["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "gael.grammar_check"));
}

#[tokio::test]
async fn tools_call_returns_structured_issues() {
    let app = app();
    let frame = json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"gael.grammar_check","arguments":{"text":"Tá an peann ar an mbord"}}
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let sc = v["result"]["structuredContent"].as_array().expect("structuredContent");
    assert!(!sc.is_empty());
    assert!(sc[0]["json"]["issues"].is_array());
}

#[tokio::test]
async fn get_mcp_sse_negotiates_event_stream() {
    let app = app();

    let req = Request::builder()
        .method("GET")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::ACCEPT, "text/event-stream")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let ctype = res.headers().get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
    assert!(ctype.starts_with("text/event-stream"));
}

// Optional live upstream check; set GRAMADOIR_BASE_URL
#[tokio::test]
#[ignore]
async fn external_gramadoir_smoke() {
    let base = std::env::var("GRAMADOIR_BASE_URL").expect("set GRAMADOIR_BASE_URL");
    let checker = Arc::new(irish_mcp_gateway::clients::gramadoir::GramadoirRemote::new(base))
        as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let svc = mcp::make_streamable_http_service(move || mcp::factory_with_checker(checker.clone()));
    let app = Router::new().route_service("/mcp", any_service(svc));

    let frame = json!({
        "jsonrpc":"2.0","id":9,"method":"tools/call",
        "params":{"name":"gael.grammar_check","arguments":{"text":"Tá an peann ar an mbord"}}
    });

    let req = Request::builder()
        .method("POST").uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(frame.to_string())).unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["result"]["structuredContent"][0]["json"]["issues"].is_array());
}

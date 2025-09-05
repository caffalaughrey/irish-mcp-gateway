use std::sync::Arc;
use std::sync::Once;

use axum::{routing::any_service, Router};
use http_body_util::BodyExt; // for .collect
use hyper::{header, Request, StatusCode};
use rmcp::model::JsonObject;
use rmcp::{ErrorData as McpError, Json as McpJson};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};
use tower::ServiceExt; // for .oneshot

use irish_mcp_gateway::infra::mcp;

const MCP_PROTOCOL_VERSION: &str = "0.5";

static INIT: Once = Once::new();
fn init_tracing() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt::try_init();
        println!("tracing_subscriber initialized");
    });
}

#[derive(Debug, serde::Deserialize)]
struct StreamData {
    event: String,
    data: serde_json::Value,
}

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

#[tokio::test]
async fn initialize_returns_server_info_and_capabilities() {
    init_tracing();
    let checker = Arc::new(FakeChecker) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let factory = move || mcp::factory_with_checker(checker.clone());
    let session_mgr = Arc::new(mcp::LocalSessionManager::default());
    let app = mcp::make_streamable_http_service(factory, session_mgr.clone());

    // Perform initialization handshake and get session ID
    let frame = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "test_client",
                "version": "0.1.0"
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    let res_status = res.status();
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        res_status.is_success(),
        "status={} body={}",
        res_status,
        body_str
    );
    let json_str = body_str.strip_prefix("data: ").unwrap_or(&body_str);
    let v: Value = serde_json::from_str(&json_str).unwrap();
    assert!(v["result"]["serverInfo"].is_object());
    assert!(v["result"]["capabilities"].is_object());
}

#[tokio::test]
async fn tools_list_includes_grammar_check() {
    init_tracing();
    let checker = Arc::new(FakeChecker) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let factory = move || mcp::factory_with_checker(checker.clone());
    let session_mgr = Arc::new(mcp::LocalSessionManager::default());
    let app = mcp::make_streamable_http_service(factory.clone(), session_mgr.clone());

    // Send an initialize request first
    let init_frame = json!({
        "jsonrpc":"2.0","id":0,"method":"initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "test_client",
                "version": "0.1.0"
            }
        }
    });
    let init_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(init_frame.to_string()))
        .unwrap();
    let init_res = app.clone().oneshot(init_req).await.unwrap();
    let session_id = init_res
        .headers()
        .get("MCP-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let init_res_status = init_res.status();
    let init_body_bytes = init_res.into_body().collect().await.unwrap().to_bytes();
    let init_body_str = String::from_utf8_lossy(&init_body_bytes);
    assert!(init_res_status.is_success());
    let _init_json_str = init_body_str
        .strip_prefix("data: ")
        .unwrap_or(&init_body_str);

    // Send the required "initialized" notification to complete handshake
    let initialized_notif = json!({
        "jsonrpc":"2.0",
        "method":"notifications/initialized",
        "params": {}
    });
    let initialized_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(initialized_notif.to_string()))
        .unwrap();
    let initialized_res = app.clone().oneshot(initialized_req).await.unwrap();
    println!(
        "initialized notification status: {}",
        initialized_res.status()
    );
    assert_eq!(initialized_res.status(), StatusCode::ACCEPTED);

    // 2. POST /mcp tools/list expecting SSE response body with rpcResponse
    let frame = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
    let req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    let res = timeout(Duration::from_secs(30), app.clone().oneshot(req))
        .await
        .expect("tools/list POST timed out")
        .unwrap();
    assert!(res.status().is_success());
    assert!(res
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
    let body = res.into_body();
    let bytes = http_body_util::BodyExt::collect(body)
        .await
        .unwrap()
        .to_bytes();
    let s = String::from_utf8_lossy(&bytes);
    println!("SSE raw body for tools/list:\n{}", s);
    let v = s
        .lines()
        .find_map(|line| line.strip_prefix("data: ").map(|d| d.to_string()))
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .expect("Did not find an rpcResponse for tools/list");
    let tools = v["result"]["tools"].as_array().expect("tools array");
    assert!(
        tools
            .iter()
            .any(|tool| tool["name"] == "gael.grammar_check"),
        "missing tool 'gael.grammar_check', got: {:?}",
        tools
    );
}

#[tokio::test]
async fn tools_call_returns_structured_issues() {
    init_tracing();
    let checker = Arc::new(FakeChecker) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let factory = move || mcp::factory_with_checker(checker.clone());
    let session_mgr = Arc::new(mcp::LocalSessionManager::default());
    let app = mcp::make_streamable_http_service(factory.clone(), session_mgr.clone());

    // Send an initialize request first
    let init_frame = json!({
        "jsonrpc":"2.0","id":0,"method":"initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "test_client",
                "version": "0.1.0"
            }
        }
    });
    let init_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(init_frame.to_string()))
        .unwrap();
    let init_res = app.clone().oneshot(init_req).await.unwrap();
    let session_id = init_res
        .headers()
        .get("MCP-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let init_res_status = init_res.status();
    let init_body_bytes = init_res.into_body().collect().await.unwrap().to_bytes();
    let init_body_str = String::from_utf8_lossy(&init_body_bytes);
    assert!(init_res_status.is_success());
    let _init_json_str = init_body_str
        .strip_prefix("data: ")
        .unwrap_or(&init_body_str);

    // Send the required "initialized" notification to complete handshake
    let initialized_notif = json!({
        "jsonrpc":"2.0",
        "method":"notifications/initialized",
        "params": {}
    });
    let initialized_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(initialized_notif.to_string()))
        .unwrap();
    let initialized_res = app.clone().oneshot(initialized_req).await.unwrap();
    println!(
        "initialized notification status: {}",
        initialized_res.status()
    );
    assert_eq!(initialized_res.status(), StatusCode::ACCEPTED);

    // 2. POST /mcp tools/call (expect SSE body with rpcResponse)
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
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(frame.to_string()))
        .unwrap();

    // Send the POST request and consume its SSE body directly
    let sse_res = app
        .clone()
        .oneshot(req)
        .await
        .expect("Tools call POST request failed");
    assert!(sse_res.status().is_success());
    let body = sse_res.into_body();
    let bytes = http_body_util::BodyExt::collect(body)
        .await
        .unwrap()
        .to_bytes();
    let s = String::from_utf8_lossy(&bytes);
    println!("SSE raw body for tools/call:\n{}", s);
    let v = s
        .lines()
        .find_map(|line| line.strip_prefix("data: ").map(|d| d.to_string()))
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d).ok())
        .expect("Did not find an rpcResponse for tools/call");
    dbg!(&v);

    let json_rpc_response = v.as_object().expect("rpc response object");
    // Extract structured content JSON payload: result.structuredContent.issues
    let issues = json_rpc_response["result"]["structuredContent"]["issues"]
        .as_array()
        .expect("issues array");
    assert!(!issues.is_empty(), "expected at least one dummy issue");
    assert_eq!(issues[0]["code"], "TEST");
    assert_eq!(issues[0]["message"].as_str().unwrap(), "ok");
}

#[tokio::test]
async fn get_mcp_sse_negotiates_event_stream() {
    init_tracing();
    timeout(Duration::from_secs(20), async move {
        let checker = Arc::new(FakeChecker) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
        let factory = move || mcp::factory_with_checker(checker.clone());
        let session_mgr = Arc::new(mcp::LocalSessionManager::default());
        let app = mcp::make_streamable_http_service(factory.clone(), session_mgr.clone());

        // Send an initialize request first
        let init_frame = json!({
            "jsonrpc":"2.0","id":0,"method":"initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "test_client",
                    "version": "0.1.0"
                }
            }
        });
        let init_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header(header::CONTENT_TYPE, "application/json")
            .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
            .body(axum::body::Body::from(init_frame.to_string()))
            .unwrap();
        let init_res = app.clone().oneshot(init_req).await.unwrap();
        let session_id = init_res
            .headers()
            .get("MCP-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .expect("session id header");

        // 2. GET /mcp event stream
        let req = Request::get(format!("/mcp"))
            .header(header::ACCEPT, "text/event-stream, application/json")
            .header("MCP-Session-Id", session_id.clone())
            .body(axum::body::Body::empty())
            .unwrap();

        let res = timeout(Duration::from_secs(10), app.clone().oneshot(req)).await;
        let res = res.expect("GET request timed out").unwrap();
        let res_status = res.status();
        let ctype_header_value = res.headers().get(header::CONTENT_TYPE);
        let ctype = ctype_header_value.map(|v| v.to_str().unwrap().to_owned());
        // Simplified SSE stream verification: only check headers and status
        // The client library is expected to handle the actual SSE stream parsing.

        assert!(res_status.is_success());
        assert!(ctype.is_some() && ctype.unwrap().starts_with("text/event-stream"));
    })
    .await
    .expect("Test timed out");
}

// Optional live upstream check; set GRAMADOIR_BASE_URL
#[tokio::test]
#[ignore]
async fn external_gramadoir_smoke() {
    let base = std::env::var("GRAMADOIR_BASE_URL").expect("set GRAMADOIR_BASE_URL");
    let checker = Arc::new(irish_mcp_gateway::clients::gramadoir::GramadoirRemote::new(
        base,
    )) as Arc<dyn mcp::GrammarCheck + Send + Sync>;
    let session_mgr = Arc::new(mcp::LocalSessionManager::default());
    let svc = mcp::make_streamable_http_service(
        move || mcp::factory_with_checker(checker.clone()),
        session_mgr,
    );
    let app = Router::new().route_service("/mcp", any_service(svc));

    let frame = json!({
        "jsonrpc":"2.0","id":9,"method":"tools/call",
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

    let res = app.oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["result"]["structuredContent"][0]["json"]["issues"].is_array());
}

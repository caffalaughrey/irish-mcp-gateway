use std::sync::Arc;

use axum::{routing::any_service, Router};
use http_body_util::BodyExt; // for .collect
use hyper::{header, Request, StatusCode};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};
use tower::ServiceExt; // for .oneshot

use irish_mcp_gateway::infra::runtime::mcp_transport;

static MCP_PROTOCOL_VERSION: &str = "0.5";

#[tokio::test]
async fn v2_initialize_list_and_call_using_pure_transport_and_tool_router() {
    // Build a Router<Service> using the refactored transport and grammar tool router
    let server = httpmock::MockServer::start();
    server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/api/gramadoir/1.0")
            .json_body(json!({"teacs":"Tá an peann ar an mbord"}));
        then.status(200).json_body(json!([{
            "context":"Tá an peann ar an mbord","contextoffset":"0","errorlength":"2","fromx":"0","fromy":"0","msg":"Agreement","ruleId":"AGR","tox":"2","toy":"0"
        }]));
    });

    let factory = {
        let base = server.base_url();
        move || {
            let svc = irish_mcp_gateway::tools::grammar::tool_router::GrammarSvc {
                checker: irish_mcp_gateway::clients::gramadoir::GramadoirRemote::new(base.clone()),
            };
            let tools: irish_mcp_gateway::tools::grammar::tool_router::GrammarRouter =
                irish_mcp_gateway::tools::grammar::tool_router::GrammarSvc::router();
            (svc, tools)
        }
    };

    let session_mgr = Arc::new(mcp_transport::LocalSessionManager::default());
    let app = mcp_transport::make_streamable_http_service(factory, session_mgr);
    let app = Router::new().route_service("/mcp", any_service(app));

    // Initialize
    let init = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{ "protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"} }
    });
    let init_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION)
        .body(axum::body::Body::from(init.to_string()))
        .unwrap();
    let init_res = app.clone().oneshot(init_req).await.unwrap();
    assert!(init_res.status().is_success());
    let session_id = init_res
        .headers()
        .get("MCP-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    // notifications/initialized
    let initialized_notif =
        json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}});
    let initialized_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(initialized_notif.to_string()))
        .unwrap();
    let initialized_res = app.clone().oneshot(initialized_req).await.unwrap();
    assert_eq!(initialized_res.status(), StatusCode::ACCEPTED);

    // tools/list
    let list = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
    let list_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(list.to_string()))
        .unwrap();
    let list_res = timeout(Duration::from_secs(20), app.clone().oneshot(list_req))
        .await
        .unwrap()
        .unwrap();
    assert!(list_res.status().is_success());

    // tools/call
    let call = json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params": {"name":"gael.grammar_check","arguments":{"text":"Tá an peann ar an mbord"}}
    });
    let call_req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header(header::CONTENT_TYPE, "application/json")
        .header("MCP-Session-Id", session_id.clone())
        .body(axum::body::Body::from(call.to_string()))
        .unwrap();
    let call_res = app.clone().oneshot(call_req).await.unwrap();
    assert!(call_res.status().is_success());
    let bytes = call_res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8_lossy(&bytes);
    let v: Value = s
        .lines()
        .find_map(|line| line.strip_prefix("data: ").map(|d| d.to_string()))
        .and_then(|d| serde_json::from_str::<Value>(&d).ok())
        .expect("Did not find an rpcResponse for tools/call");
    assert!(v["result"]["structuredContent"]["issues"].is_array());
}

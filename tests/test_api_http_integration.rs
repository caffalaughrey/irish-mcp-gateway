use axum::body::{to_bytes, Body};
use axum::{routing::post, Router};
use hyper::Request;
use irish_mcp_gateway::{api::mcp, tools::registry::build_registry}; // if lib target is unavailable, inline a copy of main's router in this test
use serde_json::Value as J;
use tower::ServiceExt;

const BODY_LIMIT: usize = 1024 * 1024;

#[tokio::test]
async fn http_e2e_tools_list_and_call() {
    let app = Router::new()
        .route("/mcp", post(mcp::http))
        .with_state(build_registry());

    // list
    let list = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools.list"}"#,
        ))
        .unwrap();
    let resp = app.clone().oneshot(list).await.unwrap();
    assert!(resp.status().is_success());

    // call
    let call = Request::builder()
        .method("POST").uri("/mcp")
        .header("content-type","application/json")
        .body(Body::from(r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"spell.check","arguments":{"text":"test"}}}"#)).unwrap();
    let resp = app.clone().oneshot(call).await.unwrap();
    let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
    let v: J = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"]["corrections"], serde_json::Value::Array(vec![]));
}

use axum::body::{to_bytes, Body};
use axum::{routing::post, Router};
use hyper::Request;
use irish_mcp_gateway::api::mcp2; // new registry v2 path
use serde_json::Value as J;
use tower::ServiceExt;

const BODY_LIMIT: usize = 1024 * 1024;

#[tokio::test]
async fn it_lists_and_calls_using_registry_v2() {
    let reg = irish_mcp_gateway::tools::registry2::build_registry_v2_from_env();
    let app = Router::new()
        .route("/mcp", post(mcp2::http))
        .with_state(reg);

    // list
    let list = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"tools.list"}"#))
        .unwrap();
    let resp = app.clone().oneshot(list).await.unwrap();
    assert!(resp.status().is_success());

    // call the new v2 spellcheck placeholder
    let call = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type","application/json")
        .body(Body::from(r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.spellcheck.v1","arguments":{"text":"Dia"}}}"#)).unwrap();
    let resp = app.clone().oneshot(call).await.unwrap();
    let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
    let v: J = serde_json::from_slice(&bytes).unwrap();
    assert!(v["result"]["corrections"].is_array());
}



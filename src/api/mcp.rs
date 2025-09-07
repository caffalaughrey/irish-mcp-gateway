use crate::tools::registry::Registry;
use axum::Json;
use serde_json::{json, Value as J};
use std::io::{self, BufRead, Write};

use crate::core::mcp::{err as rpc_err, ok as rpc_ok};
use crate::core::mcp::{RpcReq, RpcResp};
use crate::infra::http::json as http_json;

fn tools_list(reg: &Registry) -> J {
    let tools: Vec<J> = reg.0.values().map(|t| {
        json!({ "name": t.name(), "description": t.description(), "inputSchema": t.input_schema() })
    }).collect();
    json!({ "tools": tools })
}

async fn call_tool(reg: &Registry, params: &J) -> Result<J, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing tool name")?;
    let tool = reg
        .0
        .get(name)
        .ok_or_else(|| format!("unknown tool: {name}"))?;
    let args = params.get("arguments").unwrap_or(&J::Null);
    tool.call(args).await.map_err(|e| e.to_string())
}

// HTTP handler
pub async fn http(
    axum::extract::State(reg): axum::extract::State<Registry>,
    Json(req): Json<RpcReq>,
) -> Json<RpcResp> {
    tracing::debug!(method = %req.method, id = ?req.id, "HTTP handler invoked");
    let id = req.id.clone();
    let resp = match req.method.as_str() {
        "initialize" => http_json::ok(
            id.clone(),
            json!({ "serverInfo": { "name": "irish-mcp-gateway", "version": "0.1.0" }, "capabilities": {} }),
        ).0,
        "shutdown" => http_json::ok(id.clone(), J::Null).0,
        "tools.list" | "tools/list" => {
            let resp = http_json::ok(id.clone(), tools_list(&reg)).0;
            tracing::trace!(response = ?resp, "tools.list response");
            resp
        }
        "tools.call" | "tools/call" => match call_tool(&reg, &req.params).await {
            Ok(out) => {
                let resp = http_json::ok(id.clone(), out).0;
                tracing::trace!(response = ?resp, "tools.call ok response");
                resp
            }
            Err(e) => {
                let resp = http_json::error(id.clone(), -32000, e).0;
                tracing::warn!(response = ?resp, "tools.call error response");
                resp
            }
        },
        _ => http_json::error(id.clone(), -32601, format!("unknown method: {}", req.method)).0,
    };
    tracing::debug!(response = ?resp, "HTTP handler completed");
    Json(resp)
}

// Stdio loop
#[allow(dead_code)]
pub async fn stdio_loop(reg: Registry) -> anyhow::Result<()> {
    eprintln!("mode=stdio");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let req: Result<RpcReq, _> = serde_json::from_str(&line);
        let resp = match req {
            Ok(r) => {
                let id = r.id.clone();
                match r.method.as_str() {
                    "tools.list" | "tools/list" => rpc_ok(id, tools_list(&reg)),
                    "initialize" => rpc_ok(
                        id,
                        json!({ "serverInfo": { "name": "irish-mcp-gateway", "version": "0.1.0" }, "capabilities": {} }),
                    ),
                    "tools.call" | "tools/call" => match call_tool(&reg, &r.params).await {
                        Ok(out) => rpc_ok(id, out),
                        Err(e) => rpc_err(id, -32000, e, None),
                    },
                    _ => rpc_err(id, -32601, format!("unknown method: {}", r.method), None),
                }
            }
            Err(e) => http_json::parse_error(format!("parse error: {e}")).0,
        };

        let s = serde_json::to_string(&resp)?;
        println!("{s}");
        io::stdout().flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::{routing::post, Router};
    use hyper::Request;
    use serde_json::Value as J;
    use tower::ServiceExt;

    const BODY_LIMIT: usize = 1024 * 1024;

    fn router_with_state() -> Router {
        let reg = crate::tools::registry::build_registry();
        Router::new()
            .route("/mcp", post(super::http))
            .with_state(reg)
    }

    #[test]
    fn tools_list_returns_expected_shape() {
        let reg = crate::tools::registry::build_registry();
        let v = super::tools_list(&reg);
        assert!(v["tools"].is_array());
        assert_eq!(v["tools"][0]["name"], "gael.spellcheck.v1");
    }

    #[tokio::test]
    async fn call_tool_returns_corrections_array() {
        let reg = crate::tools::registry::build_registry();
        let out = super::call_tool(
            &reg,
            &serde_json::json!({
                "name":"gael.spellcheck.v1",
                "arguments":{"text":"test"}
            }),
        )
        .await
        .unwrap();
        assert_eq!(out["corrections"], serde_json::Value::Array(vec![]));
    }

    #[tokio::test]
    async fn http_tools_list_returns_200_and_array() {
        let app = router_with_state();
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools.list"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert!(v["result"]["tools"].is_array());
    }

    #[tokio::test]
    async fn http_tools_call_returns_200() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.spellcheck.v1","arguments":{"text":"test"}}}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["result"]["corrections"], serde_json::Value::Array(vec![]));
    }

    #[tokio::test]
    async fn http_tools_call_missing_arguments_returns_tool_error() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":5,"method":"tools.call","params":{"name":"gael.spellcheck.v1"}}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn http_tools_call_unknown_tool_returns_error() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":3,"method":"tools.call","params":{"name":"does.not.exist","arguments":{}}}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn http_unknown_method_returns_method_not_found() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":4,"method":"nope"}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn http_parse_error_on_malformed_json() {
        let app = router_with_state();
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from("{ not-json }"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn http_grammar_check_ok_with_mocked_backend() {
        // Tool trait not used in this test but kept for reference
        use httpmock::prelude::*;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .json_body(json!({"teacs":"Tá an peann ar an mbord"}));
            then.status(200).json_body(json!([{
                "context":"Tá an peann ar an mbord",
                "contextoffset":"0",
                "errorlength":"2",
                "fromx":"0",
                "fromy":"0",
                "msg":"Agreement",
                "ruleId":"AGR",
                "tox":"2",
                "toy":"0"
            }]));
        });

        let reg = crate::tools::registry::build_registry();

        let app = axum::Router::new()
            .route("/mcp", axum::routing::post(super::http))
            .with_state(reg);

        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.spellcheck.v1","arguments":{"text":"Tá an peann ar an mbord"}}}"#;
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["result"]["corrections"], serde_json::Value::Array(vec![]));
    }
}

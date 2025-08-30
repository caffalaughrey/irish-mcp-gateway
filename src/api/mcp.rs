use std::io::{self, BufRead, Write};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as J};
use crate::tools::registry::Registry;

#[derive(Deserialize)]
pub struct RpcReq { pub jsonrpc: String, pub id: J, pub method: String, #[serde(default)] pub params: J }

#[derive(Serialize)]
pub struct RpcResp {
    pub jsonrpc: &'static str, pub id: J,
    #[serde(skip_serializing_if = "Option::is_none")] pub result: Option<J>,
    #[serde(skip_serializing_if = "Option::is_none")] pub error: Option<RpcErr>,
}
#[derive(Serialize)]
pub struct RpcErr { pub code: i32, pub message: String, #[serde(skip_serializing_if = "Option::is_none")] pub data: Option<J> }

fn ok(id: J, result: J) -> RpcResp { RpcResp { jsonrpc: "2.0", id, result: Some(result), error: None } }
fn err(id: J, code: i32, msg: impl Into<String>, data: Option<J>) -> RpcResp {
    RpcResp { jsonrpc: "2.0", id, result: None, error: Some(RpcErr { code, message: msg.into(), data }) }
}

fn tools_list(reg: &Registry) -> J {
    let tools: Vec<J> = reg.0.values().map(|t| {
        json!({ "name": t.name(), "description": t.description(), "inputSchema": t.input_schema() })
    }).collect();
    json!({ "tools": tools })
}

async fn call_tool(reg: &Registry, params: &J) -> Result<J, String> {
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing tool name")?;
    let tool = reg.0.get(name).ok_or_else(|| format!("unknown tool: {name}"))?;
    let args = params.get("arguments").unwrap_or(&J::Null);
    tool.call(args).await.map_err(|e| e.to_string())
}

// HTTP handler
pub async fn http(
    axum::extract::State(reg): axum::extract::State<Registry>,
    Json(req): Json<RpcReq>,
) -> Json<RpcResp> {
    let id = req.id.clone();
    let resp = match req.method.as_str() {
        "tools.list" | "tools/list" => ok(id, tools_list(&reg)),
        "tools.call" | "tools/call" => match call_tool(&reg, &req.params).await {
            Ok(out) => ok(id, out),
            Err(e) => err(id, -32000, e, None),
        },
        _ => err(id, -32601, format!("unknown method: {}", req.method), None),
    };
    Json(resp)
}

// Stdio loop
pub async fn stdio_loop(reg: Registry) -> anyhow::Result<()> {
    eprintln!("mode=stdio");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        if line.trim().is_empty() { continue; }

        let req: Result<RpcReq, _> = serde_json::from_str(&line);
        let resp = match req {
            Ok(r) => {
                let id = r.id.clone();
                match r.method.as_str() {
                    "tools.list" | "tools/list" => ok(id, tools_list(&reg)),
                    "tools.call" | "tools/call" => match call_tool(&reg, &r.params).await {
                        Ok(out) => ok(id, out),
                        Err(e) => err(id, -32000, e, None),
                    },
                    _ => err(id, -32601, format!("unknown method: {}", r.method), None),
                }
            }
            Err(e) => RpcResp { jsonrpc: "2.0", id: J::Null, result: None,
                error: Some(RpcErr { code: -32700, message: format!("parse error: {e}"), data: None }) },
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
    use axum::{Router, routing::post};
    use hyper::Request;
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt; // for `oneshot`

    const BODY_LIMIT: usize = 1024 * 1024;

    fn router_with_state() -> Router {
        let reg = crate::tools::registry::build_registry();
        Router::new().route("/mcp", post(super::http)).with_state(reg)
    }

    #[test]
    fn it_knows_tools_list_shape() {
        let reg = crate::tools::registry::build_registry();
        let v = super::tools_list(&reg);
        assert!(v["tools"].is_array());
        assert_eq!(v["tools"][0]["name"], "hello.echo");
    }

    #[tokio::test]
    async fn it_knows_call_tool_happy_path() {
        let reg = crate::tools::registry::build_registry();
        let out = super::call_tool(&reg, &serde_json::json!({
            "name":"hello.echo",
            "arguments":{"name":"Arn"}
        })).await.unwrap();
        assert_eq!(out["message"], "Dia dhuit, Arn!");
    }

    #[tokio::test]
    async fn it_knows_http_tools_list() {
        let app = router_with_state();
        let req = Request::builder()
            .method("POST").uri("/mcp")
            .header("content-type","application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"tools.list"}"#)).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert!(v["result"]["tools"].is_array());
    }

    #[tokio::test]
    async fn it_knows_http_tools_call_ok() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"hello.echo","arguments":{"name":"Arn"}}}"#;
        let req = Request::builder()
            .method("POST").uri("/mcp")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["result"]["message"], "Dia dhuit, Arn!");
    }

    #[tokio::test]
    async fn it_knows_http_unknown_tool() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":3,"method":"tools.call","params":{"name":"does.not.exist","arguments":{}}}"#;
        let req = Request::builder()
            .method("POST").uri("/mcp")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn it_knows_http_unknown_method() {
        let app = router_with_state();
        let body = r#"{"jsonrpc":"2.0","id":4,"method":"nope"}"#;
        let req = Request::builder()
            .method("POST").uri("/mcp")
            .header("content-type","application/json")
            .body(Body::from(body)).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = to_bytes(resp.into_body(), BODY_LIMIT).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }
}

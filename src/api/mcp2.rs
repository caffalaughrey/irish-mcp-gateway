use axum::Json;
use serde_json::{json, Value as J};

use crate::core::mcp::{RpcReq, RpcResp};
use crate::infra::http::json as http_json;
use crate::tools::registry2::ToolRegistry;

fn tools_list(reg: &ToolRegistry) -> J {
    let tools: Vec<J> = reg
        .list()
        .into_iter()
        .map(|t| json!({ "name": t.name, "description": t.description, "inputSchema": t.input_schema }))
        .collect();
    json!({ "tools": tools })
}

async fn call_tool(reg: &ToolRegistry, params: &J) -> Result<J, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing tool name")?;
    let args = params.get("arguments").unwrap_or(&J::Null).clone();
    reg.call(name, &args).await
}

pub async fn http(
    axum::extract::State(reg): axum::extract::State<ToolRegistry>,
    Json(req): Json<RpcReq>,
) -> Json<RpcResp> {
    let id = req.id.clone();
    let resp = match req.method.as_str() {
        "initialize" => http_json::ok(
            id.clone(),
            json!({ "serverInfo": { "name": "irish-mcp-gateway", "version": "0.1.0" }, "capabilities": {} }),
        ).0,
        "shutdown" => http_json::ok(id.clone(), J::Null).0,
        "tools.list" | "tools/list" => http_json::ok(id.clone(), tools_list(&reg)).0,
        "tools.call" | "tools/call" => match call_tool(&reg, &req.params).await {
            Ok(out) => http_json::ok(id.clone(), out).0,
            Err(e) => http_json::error(id.clone(), -32000, e).0,
        },
        _ => http_json::error(id.clone(), -32601, format!("unknown method: {}", req.method)).0,
    };
    Json(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::{routing::post, Router};
    use hyper::Request;
    use serde_json::Value as J;
    use tower::ServiceExt;

    #[tokio::test]
    async fn it_lists_and_calls_using_registry_v2() {
        let reg = crate::tools::registry2::build_registry_v2_from_env();
        let app = Router::new().route("/mcp", post(super::http)).with_state(reg);

        // list
        let list = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"tools.list"}"#))
            .unwrap();
        let resp = app.clone().oneshot(list).await.unwrap();
        assert!(resp.status().is_success());

        // call spellcheck placeholder (always present)
        let call = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.spellcheck.v1","arguments":{"text":"Dia"}}}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(call).await.unwrap();
        let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        let v: J = serde_json::from_slice(&bytes).unwrap();
        assert!(v["result"]["corrections"].is_array());
    }
}



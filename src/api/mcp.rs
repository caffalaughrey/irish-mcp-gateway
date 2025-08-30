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

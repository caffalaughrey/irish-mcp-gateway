use std::{net::SocketAddr, io::{self, BufRead, Write}};
use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as J};

#[derive(Deserialize)]
struct RpcReq {
    jsonrpc: String,
    id: J,
    method: String,
    #[serde(default)]
    params: J,
}

#[derive(Serialize)]
struct RpcResp {
    jsonrpc: &'static str,
    id: J,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<J>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcErr>,
}

#[derive(Serialize)]
struct RpcErr {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<J>,
}

fn ok(id: J, result: J) -> RpcResp {
    RpcResp { jsonrpc: "2.0", id, result: Some(result), error: None }
}
fn err(id: J, code: i32, msg: impl Into<String>, data: Option<J>) -> RpcResp {
    RpcResp { jsonrpc: "2.0", id, result: None, error: Some(RpcErr { code, message: msg.into(), data }) }
}

fn tools_list() -> J {
    json!({
      "tools": [{
        "name": "hello.echo",
        "description": "Return a friendly greeting",
        "inputSchema": {
          "type":"object",
          "properties": { "name": { "type":"string" } },
          "required": []
        }
      }]
    })
}

fn call_hello_echo(params: &J) -> Result<J, String> {
    let name = params.get("arguments")
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("world");
    Ok(json!({ "message": format!("Dia dhuit, {name}!") }))
}

async fn mcp_http(Json(req): Json<RpcReq>) -> Json<RpcResp> {
    let resp = match req.method.as_str() {
        "tools.list" | "tools/list" => ok(req.id, tools_list()),
        "tools.call" | "tools/call" => {
            let tool = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match tool {
                "hello.echo" => match call_hello_echo(&req.params) {
                    Ok(r) => ok(req.id, r),
                    Err(e) => err(req.id, -32000, e, None),
                },
                _ => err(req.id, -32601, format!("unknown tool: {tool}"), None),
            }
        }
        _ => err(req.id, -32601, format!("unknown method: {}", req.method), None),
    };
    Json(resp)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("BOOT hello-mcp (minimal)");

    // stdio mode (reads newline-delimited JSON; exits on EOF)
    if std::env::var("MODE").map(|v| v.eq_ignore_ascii_case("stdio")).unwrap_or(false) {
        eprintln!("mode=stdio");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }

            let req: Result<RpcReq, _> = serde_json::from_str(&line);
            let resp = match req {
                Ok(r) => {
                    let id = r.id.clone(); // keep the original request id
                    match r.method.as_str() {
                        "tools.list" | "tools/list" => ok(id, tools_list()),
                        "tools.call" | "tools/call" => {
                            let tool = r.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            match tool {
                                "hello.echo" => match call_hello_echo(&r.params) {
                                    Ok(out) => ok(id, out),
                                    Err(e)  => err(id, -32000, e, None),
                                },
                                _ => err(id, -32601, format!("unknown tool: {tool}"), None),
                            }
                        }
                        _ => err(id, -32601, format!("unknown method: {}", r.method), None),
                    }
                }
                Err(e) => RpcResp {
                    jsonrpc: "2.0",
                    id: serde_json::Value::Null,
                    result: None,
                    error: Some(RpcErr { code: -32700, message: format!("parse error: {e}"), data: None }),
                },
            };

            let s = serde_json::to_string(&resp)?;
            println!("{s}");
            io::stdout().flush()?;
        }
        return Ok(());
    }

    // server mode (HTTP POST /mcp)
    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
    let addr: SocketAddr = ([0,0,0,0], port).into();

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/mcp", post(mcp_http));

    eprintln!("mode=server port={}", port);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

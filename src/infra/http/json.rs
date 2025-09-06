use axum::Json;
use serde_json::json;

use crate::core::mcp::{err as rpc_err, ok as rpc_ok, RpcErr, RpcReq, RpcResp};

pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Json<RpcResp> {
    Json(rpc_ok(id, result))
}

pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Json<RpcResp> {
    Json(rpc_err(id, code, message, None))
}

pub fn parse_error(message: impl Into<String>) -> Json<RpcResp> {
    Json(RpcResp { jsonrpc: "2.0", id: serde_json::Value::Null, result: None, error: Some(RpcErr { code: -32700, message: message.into(), data: None }) })
}



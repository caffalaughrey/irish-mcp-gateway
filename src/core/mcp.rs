//! Shared MCP protocol surface (lightweight shim to avoid tool coupling)

use serde::{Deserialize, Serialize};
use serde_json::Value as J;

// --- JSON-RPC structures used by the deprecated REST shim and tests ---

#[derive(Deserialize, Debug)]
pub struct RpcReq {
    pub jsonrpc: String,
    pub id: J,
    pub method: String,
    #[serde(default)]
    pub params: J,
}

#[derive(Serialize, Debug, Clone)]
pub struct RpcResp {
    pub jsonrpc: &'static str,
    pub id: J,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<J>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErr>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RpcErr {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<J>,
}

pub fn ok(id: J, result: J) -> RpcResp {
    RpcResp { jsonrpc: "2.0", id, result: Some(result), error: None }
}
pub fn err(id: J, code: i32, msg: impl Into<String>, data: Option<J>) -> RpcResp {
    RpcResp { jsonrpc: "2.0", id, result: None, error: Some(RpcErr { code, message: msg.into(), data }) }
}

// --- Minimal Initialize result for tests/docs ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitializeResult {
    pub server_info: ServerInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_serializes_initialize_result() {
        let v = InitializeResult { server_info: ServerInfo { name: "gw".into(), version: "0.1".into() } };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("server_info"));
    }
}



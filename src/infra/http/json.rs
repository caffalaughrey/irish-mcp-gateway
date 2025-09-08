use axum::Json;

use crate::core::mcp::{err as rpc_err, ok as rpc_ok, RpcErr, RpcResp};
use crate::core::error::GatewayError;

pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Json<RpcResp> {
    Json(rpc_ok(id, result))
}

pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Json<RpcResp> {
    Json(rpc_err(id, code, message, None))
}

pub fn parse_error(message: impl Into<String>) -> Json<RpcResp> {
    Json(RpcResp {
        jsonrpc: "2.0",
        id: serde_json::Value::Null,
        result: None,
        error: Some(RpcErr {
            code: -32700,
            message: message.into(),
            data: None,
        }),
    })
}

/// Map a GatewayError into a JSON-RPC error response (-32000 application error)
pub fn from_gateway_error(id: serde_json::Value, err: GatewayError) -> Json<RpcResp> {
    error(id, -32000, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json as AxumJson;
    use serde_json::{json, Value};

    #[test]
    fn wraps_ok_response_in_json_rpc_envelope() {
        let AxumJson(resp) = ok(json!(1), json!({"x": 1}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap()["x"], 1);
    }

    #[test]
    fn wraps_error_response_in_json_rpc_envelope() {
        let AxumJson(resp) = error(Value::Null, -32601, "method not found");
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("method not found"));
    }

    #[test]
    fn builds_parse_error_with_standard_code() {
        let AxumJson(resp) = parse_error("bad json");
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32700);
        assert!(err.message.contains("bad json"));
    }
}

use axum::{
    routing::{any_service, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::infra::runtime::mcp_transport;
use crate::tools::registry::Registry;

/// Enhanced health check endpoint with service status
async fn health_check() -> Json<Value> {
    let mut status = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "services": {}
    });

    // Check grammar service if configured
    if let Ok(grammar_url) = std::env::var("GRAMADOIR_BASE_URL") {
        if !grammar_url.is_empty() {
            let client = crate::clients::gramadoir::GramadoirRemote::new(grammar_url);
            match client.analyze("test").await {
                Ok(_) => {
                    status["services"]["grammar"] = json!({
                        "status": "healthy",
                        "url": std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default()
                    });
                }
                Err(_) => {
                    status["services"]["grammar"] = json!({
                        "status": "unhealthy",
                        "url": std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default()
                    });
                    status["status"] = json!("degraded");
                }
            }
        }
    }

    Json(status)
}

/// Default, spec-compliant app: `/healthz` + streamable MCP at `/mcp`.
pub fn build_app_default() -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let factory = || {
        let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
        let handler = crate::tools::grammar::tool_router::GrammarSvc {
            checker: crate::clients::gramadoir::GramadoirRemote::new(base),
        };
        let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
        (handler, tools)
    };
    let mcp_service = mcp_transport::make_streamable_http_service(factory, session_mgr);

    Router::new()
        .route("/healthz", get(health_check))
        .route_service("/mcp", any_service(mcp_service))
}

/// Spec app **plus** deprecated demo REST route at `/v1/grammar/check`.
pub fn build_app_with_deprecated_api(registry: Registry) -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let factory = || {
        let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
        let handler = crate::tools::grammar::tool_router::GrammarSvc {
            checker: crate::clients::gramadoir::GramadoirRemote::new(base),
        };
        let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
        (handler, tools)
    };
    let mcp_service = mcp_transport::make_streamable_http_service(factory, session_mgr);

    Router::new()
        .route("/healthz", get(health_check))
        .route_service("/mcp", any_service(mcp_service))
        .route("/v1/grammar/check", post(crate::api::mcp::http))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use serial_test::serial;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn healthz_responds_ok_on_default_app() {
        let app = build_app_default();
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/healthz")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Check response body is JSON
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Status can be "healthy" or "degraded" depending on grammar service availability
        assert!(matches!(
            json["status"].as_str(),
            Some("healthy") | Some("degraded")
        ));
        assert!(json["timestamp"].is_string());
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn healthz_returns_structured_response() {
        let app = build_app_default();
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/healthz")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Verify required fields
        assert!(json["status"].is_string());
        assert!(json["timestamp"].is_string());
        assert!(json["version"].is_string());
        assert!(json["services"].is_object());
    }

    #[tokio::test]
    #[serial]
    async fn healthz_indicates_grammar_healthy() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/gramadoir/1.0");
            then.status(200).json_body(serde_json::json!([]));
        });

        std::env::set_var("GRAMADOIR_BASE_URL", server.base_url());
        let app = build_app_default();
        let req = Request::builder().method("GET").uri("/healthz").body(axum::body::Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["services"]["grammar"]["status"], "healthy");
        std::env::remove_var("GRAMADOIR_BASE_URL");
    }

    #[tokio::test]
    #[serial]
    async fn healthz_indicates_grammar_unhealthy() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/gramadoir/1.0");
            then.status(500).body("boom");
        });

        std::env::set_var("GRAMADOIR_BASE_URL", server.base_url());
        let app = build_app_default();
        let req = Request::builder().method("GET").uri("/healthz").body(axum::body::Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["services"]["grammar"]["status"], "unhealthy");
        assert_eq!(json["status"], "degraded");
        std::env::remove_var("GRAMADOIR_BASE_URL");
    }

    #[tokio::test]
    async fn deprecated_route_handles_grammar_check_when_configured() {
        // Configure env so registry includes grammar tool
        std::env::set_var("GRAMADOIR_BASE_URL", "http://example");
        let reg = crate::tools::registry::build_registry();
        let app = build_app_with_deprecated_api(reg);

        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.grammar_check","arguments":{"text":"Tá an peann ar an mbord"}}}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/grammar/check")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_success());
    }

    #[tokio::test]
    async fn deprecated_route_returns_error_on_unknown_tool() {
        let reg = crate::tools::registry::build_registry();
        let app = build_app_with_deprecated_api(reg);

        let body = r#"{"jsonrpc":"2.0","id":99,"method":"tools.call","params":{"name":"does.not.exist","arguments":{}}}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/grammar/check")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn healthz_json_shape_has_required_fields() {
        let app = build_app_default();
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/healthz")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["status"].is_string());
        assert!(json["timestamp"].is_string());
        assert!(json["version"].is_string());
        assert!(json["services"].is_object());
    }
}

use axum::{
    routing::{any_service, get, post},
    Router,
};
use std::sync::Arc;

use crate::infra::runtime::mcp_transport;
use crate::tools::registry::Registry;

/// Default, spec-compliant app: `/healthz` + streamable MCP at `/mcp`.
pub fn build_app_default() -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let factory = || {
        let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
        let handler = crate::tools::grammar::tool_router::GrammarSvc { checker: crate::clients::gramadoir::GramadoirRemote::new(base) };
        let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
        (handler, tools)
    };
    let mcp_service = mcp_transport::make_streamable_http_service(factory, session_mgr);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
}

/// Spec app **plus** deprecated demo REST route at `/v1/grammar/check`.
pub fn build_app_with_deprecated_api(registry: Registry) -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let factory = || {
        let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
        let handler = crate::tools::grammar::tool_router::GrammarSvc { checker: crate::clients::gramadoir::GramadoirRemote::new(base) };
        let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
        (handler, tools)
    };
    let mcp_service = mcp_transport::make_streamable_http_service(factory, session_mgr);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
        .route("/v1/grammar/check", post(crate::api::mcp::http))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

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
}

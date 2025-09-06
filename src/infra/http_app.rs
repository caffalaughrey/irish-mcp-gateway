use axum::{
    routing::{any_service, get, post},
    Router,
};
use std::sync::Arc;

use crate::infra::mcp;
use crate::api::mcp2;
use crate::tools::registry2::ToolRegistry as ToolRegistryV2;

/// Default, spec-compliant app: `/healthz` + streamable MCP at `/mcp`.
pub fn build_app_default() -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let mcp_service = mcp::make_streamable_http_service(mcp::factory_from_env, session_mgr);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
}

/// Spec app **plus** deprecated demo REST route at `/v1/grammar/check`.
pub fn build_app_with_deprecated_api(registry: ToolRegistryV2) -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let mcp_service = mcp::make_streamable_http_service(mcp::factory_from_env, session_mgr);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
        .route("/v1/grammar/check", post(mcp2::http))
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
        // Use v2 registry and call spellcheck placeholder to avoid external dependency
        let reg = crate::tools::registry2::build_registry_v2_from_env();
        let app = build_app_with_deprecated_api(reg);

        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"gael.spellcheck.v1","arguments":{"text":"Tá an peann ar an mbord"}}}"#;
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

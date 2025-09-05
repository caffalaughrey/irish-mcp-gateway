use axum::{
    routing::{any_service, get, post},
    Router,
};
use std::sync::Arc;

use crate::infra::mcp;
use crate::tools::registry::Registry;

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
pub fn build_app_with_deprecated_api(registry: Registry) -> Router {
    let session_mgr = Arc::new(
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
    );
    let mcp_service = mcp::make_streamable_http_service(mcp::factory_from_env, session_mgr);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
        .route("/v1/grammar/check", post(crate::api::mcp::http))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
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
}

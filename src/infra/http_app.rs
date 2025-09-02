use axum::{
    routing::{any_service, get, post},
    Router,
};

use crate::infra::mcp;
use crate::tools::registry::Registry;

/// Default, spec-compliant app: `/healthz` + streamable MCP at `/mcp`.
pub fn build_app_default() -> Router {
    let mcp_service = mcp::make_streamable_http_service(mcp::factory_from_env);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
}

/// Spec app **plus** deprecated demo REST route at `/v1/grammar/check`.
pub fn build_app_with_deprecated_api(
    registry: Registry,
) -> Router {
    let mcp_service = mcp::make_streamable_http_service(mcp::factory_from_env);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route_service("/mcp", any_service(mcp_service))
        // deprecated demo route (kept for rough demo):
        .route("/v1/grammar/check", post(crate::api::mcp::http))
        .with_state(registry) // <- set Router<Registry> state once; no type mismatch
}

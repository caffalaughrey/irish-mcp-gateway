mod infra;
mod domain;
mod clients;
mod tools;
mod api;

use axum::{
    routing::{get, any_service},
    Router,
};
use infra::config::Config;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    infra::logging::init();
    let cfg = Config::from_env();
    eprintln!("BOOT irish-mcp-gateway mode={} port={}", cfg.mode, cfg.port);

    // STDIO mode: run MCP over stdio ONLY (no HTTP).
    if cfg.mode == "stdio" {
        infra::mcp::serve_stdio_from(infra::mcp::factory_from_env)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        return Ok(());
    }

    // HTTP server: keep /healthz and mount Streamable HTTP MCP at /mcp.
    let mcp_service = infra::mcp::make_streamable_http_service(infra::mcp::factory_from_env);

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        // NEW: spec-compliant MCP (POST frames + GET SSE) at the same path
        .route_service("/mcp", any_service(mcp_service));

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

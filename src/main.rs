mod infra;
mod domain;
mod clients;
mod tools;
mod api;

use std::net::SocketAddr;
use axum::{routing::{get, post}, Router};
use infra::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    infra::logging::init();

    let cfg = Config::from_env();
    eprintln!("BOOT irish-mcp-gateway mode={} port={}", cfg.mode, cfg.port);

    // Tool registry
    let registry = tools::registry::build_registry();

    // Stdio mode (optional)
    if cfg.mode == "stdio" {
        api::mcp::stdio_loop(registry).await?;
        return Ok(());
    }

    // HTTP server (MCP over /mcp + health)
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/mcp", post(api::mcp::http))
        .with_state(registry);

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

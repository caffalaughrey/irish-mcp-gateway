mod infra;
mod domain;
mod clients;
mod tools;
mod api;

use std::net::SocketAddr;
use infra::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    infra::logging::init();

    let cfg = Config::from_env();
    tracing::info!(
        mode = %cfg.mode,
        port = cfg.port,
        deprecate_rest = cfg.deprecate_rest,
        "BOOT irish-mcp-gateway"
    );

    // Stdio mode: run MCP over stdio ONLY (no HTTP).
    if cfg.mode == "stdio" {
        infra::mcp::serve_stdio_from(infra::mcp::factory_from_env)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        return Ok(());
    }

    // HTTP server
    let app = if cfg.deprecate_rest {
        // Spec-only: /healthz + streamable HTTP MCP on /mcp
        infra::http_app::build_app_default()
    } else {
        // Spec + demo REST: add /v1/grammar/check
        let registry = tools::registry::build_registry();
        infra::http_app::build_app_with_deprecated_api(registry)
    };

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

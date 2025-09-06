mod api;
mod clients;
mod domain;
mod infra;
mod tools;
mod core;

use infra::config::Config;
use std::net::SocketAddr;

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
        infra::http_app::build_app_default()
    } else {
        // Use new registry v2 for deprecated REST path
        let registry_v2 = tools::registry2::build_registry_v2_from_env();
        infra::http_app::build_app_with_deprecated_api(registry_v2)
    };

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

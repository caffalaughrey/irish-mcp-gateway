mod api;
mod cli;
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

    // Check if we're running admin commands
    if std::env::args().len() > 1 {
        let exit_code = cli::run().await;
        std::process::exit(match exit_code {
            std::process::ExitCode::SUCCESS => 0,
            _ => 1, // All other exit codes map to 1
        });
    }

    let cfg = Config::from_env();
    tracing::info!(
        mode = %cfg.mode,
        port = cfg.port,
        deprecate_rest = cfg.deprecate_rest,
        "BOOT irish-mcp-gateway"
    );

    // Stdio mode: run MCP over stdio ONLY (no HTTP).
    if cfg.mode == "stdio" {
        let factory = || {
            let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
            let handler = crate::tools::grammar::tool_router::GrammarSvc { checker: crate::clients::gramadoir::GramadoirRemote::new(base) };
            let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
            (handler, tools)
        };
        crate::infra::runtime::mcp_transport::serve_stdio(factory)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        return Ok(());
    }

    // HTTP server
    let app = if cfg.deprecate_rest {
        infra::http_app::build_app_default()
    } else {
        // Spec + demo REST: add /v1/grammar/check using legacy registry for demo only
        let registry = tools::registry::build_registry();
        infra::http_app::build_app_with_deprecated_api(registry)
    };

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

use crate::infra::config::Config;
use std::net::SocketAddr;

pub async fn run_server() -> anyhow::Result<()> {
    let cfg = Config::from_env();
    tracing::info!(
        mode = %cfg.mode,
        port = cfg.port,
        deprecate_rest = cfg.deprecate_rest,
        "BOOT irish-mcp-gateway"
    );

    if cfg.mode == "stdio" {
        let factory = || {
            let handler = crate::tools::mcp_router::UnifiedSvc;
            let tools = crate::tools::mcp_router::UnifiedSvc::router();
            (handler, tools)
        };
        crate::infra::runtime::mcp_transport::serve_stdio(factory)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        return Ok(());
    }

    let app = crate::infra::http_app::build_app_default();

    let addr: SocketAddr = ([0, 0, 0, 0], cfg.port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn app_factory_selects_server_by_default() {
        std::env::remove_var("MODE");
        let cfg = Config::from_env();
        assert_eq!(cfg.mode, "server");
    }
}

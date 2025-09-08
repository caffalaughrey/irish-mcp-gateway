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
            let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
            let handler = crate::tools::grammar::tool_router::GrammarSvc {
                checker: crate::clients::gramadoir::GramadoirRemote::new(base),
            };
            let tools = crate::tools::grammar::tool_router::GrammarSvc::router();
            (handler, tools)
        };
        crate::infra::runtime::mcp_transport::serve_stdio(factory)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        return Ok(());
    }

    let app = if cfg.deprecate_rest {
        crate::infra::http_app::build_app_default()
    } else {
        let registry = crate::tools::registry::build_registry();
        crate::infra::http_app::build_app_with_deprecated_api(registry)
    };

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

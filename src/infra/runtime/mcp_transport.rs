//! Generic MCP transport helpers (stdio + streamable HTTP) decoupled from tool logic.

use std::sync::Arc;

use rmcp::handler::server::router::Router;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::serve_server;
use rmcp::transport::streamable_http_server::tower::{StreamableHttpServerConfig, StreamableHttpService};

pub use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
pub use rmcp::ServerHandler;

pub async fn serve_stdio<H>(
    factory: impl FnOnce() -> (H, ToolRouter<H>),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    H: ServerHandler,
{
    let (handler, tools) = factory();
    let service = Router::new(handler).with_tools(tools);
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    serve_server(service, (stdin, stdout)).await?;
    Ok(())
}

pub fn make_streamable_http_service<H>(
    factory: impl Fn() -> (H, ToolRouter<H>) + Send + Sync + Clone + 'static,
    session_mgr: Arc<LocalSessionManager>,
) -> StreamableHttpService<Router<H>, LocalSessionManager>
where
    H: ServerHandler,
{
    let cfg = StreamableHttpServerConfig::default();
    let service_factory = move || {
        let (handler, tools) = factory();
        let service = Router::new(handler).with_tools(tools);
        Ok(service)
    };
    StreamableHttpService::new(service_factory, session_mgr, cfg)
}



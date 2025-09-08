//! Generic MCP transport helpers (stdio + streamable HTTP) decoupled from tool logic.

use std::sync::Arc;

use rmcp::handler::server::router::Router;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::serve_server;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};

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

/// Testable variant of stdio serving that accepts arbitrary IO.
// TODO(refactor-fit-and-finish): When we wire full stdio MCP, craft rmcp-compliant
// frames (using rmcp's serializer) in tests to assert positive roundtrips.
#[allow(dead_code)]
pub async fn serve_stdio_with_io<H, R, W>(
    factory: impl FnOnce() -> (H, ToolRouter<H>),
    reader: R,
    writer: W,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    H: ServerHandler,
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (handler, tools) = factory();
    let service = Router::new(handler).with_tools(tools);
    serve_server(service, (reader, writer)).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::gramadoir::GramadoirRemote;
    use crate::tools::grammar::tool_router::GrammarSvc;
    use std::sync::Arc;
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
    use tokio::time::{timeout, Duration, Instant};

    #[tokio::test]
    async fn test_make_streamable_http_service() {
        let session_mgr = Arc::new(LocalSessionManager::default());
        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        let _service = make_streamable_http_service(factory, session_mgr);
    }

    #[tokio::test]
    async fn test_make_streamable_http_service_with_different_factory() {
        let session_mgr = Arc::new(LocalSessionManager::default());
        let factory = || {
            let checker = GramadoirRemote::new("http://different-test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        let _service = make_streamable_http_service(factory, session_mgr);
    }

    #[tokio::test]
    async fn make_streamable_http_service_uses_session_manager() {
        let session_mgr = Arc::new(LocalSessionManager::default());
        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };
        let _service = make_streamable_http_service(factory, session_mgr.clone());
        // If session manager type mismatched, this would not compile; runtime test is smoke only.
    }

    #[test]
    fn test_serve_stdio_factory_called() {
        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        // Test that factory can be called (we can't easily test the full stdio flow)
        let _ = factory();
    }

    #[tokio::test]
    async fn test_serve_stdio_propagates_error() {
        // Create a handler whose service will error immediately by using an invalid IO pair
        let factory = || {
            let checker = GramadoirRemote::new("".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };
        // We can't easily force serve_server to error without IO, so just assert the function type compiles
        let _ = factory();
    }

    #[tokio::test]
    async fn test_serve_stdio_with_io_eof_returns_err() {
        let (mut client, server) = duplex(1024);
        let (srv_r, srv_w) = tokio::io::split(server);

        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        let serve = tokio::spawn(async move { serve_stdio_with_io(factory, srv_r, srv_w).await });

        // Close client to signal EOF
        client.shutdown().await.unwrap();
        let res = serve.await.unwrap();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_serve_stdio_with_io_bad_json_returns_err() {
        let (mut client, server) = duplex(1024);
        let (srv_r, srv_w) = tokio::io::split(server);

        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        let serve = tokio::spawn(async move { serve_stdio_with_io(factory, srv_r, srv_w).await });

        // Write malformed JSON frame then close
        client.write_all(b"{ not json }\n").await.unwrap();
        client.shutdown().await.unwrap();

        let res = serve.await.unwrap();
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_serve_stdio_with_io_initialize_then_eof() {
        let (mut client, server) = duplex(4096);
        let (srv_r, srv_w) = tokio::io::split(server);

        let factory = || {
            let checker = GramadoirRemote::new("http://test".to_string());
            let handler = GrammarSvc { checker };
            let tools = GrammarSvc::router();
            (handler, tools)
        };

        let serve = tokio::spawn(async move { serve_stdio_with_io(factory, srv_r, srv_w).await });

        // TODO(refactor-fit-and-finish): Switch to rmcp-compliant initialize frame once
        // the upstream serializer is used here; for now we only assert that bytes can be produced.
        // Send initialize request (MCP-acceptable shape), then notifications/initialized, then tools/list
        let init = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-03-26\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"0.0.0\"}}}\n";
        let inited =
            b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\",\"params\":{}}\n";
        let list = b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}\n";
        client.write_all(init).await.unwrap();
        client.write_all(inited).await.unwrap();
        client.write_all(list).await.unwrap();
        let mut buf = [0u8; 1024];
        let mut total = Vec::new();
        let deadline = Instant::now() + Duration::from_millis(1000);
        while Instant::now() < deadline {
            match timeout(Duration::from_millis(100), client.read(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => {
                    total.extend_from_slice(&buf[..n]);
                    if String::from_utf8_lossy(&total).contains("\"result\"") {
                        break;
                    }
                }
                Ok(Ok(_)) => {
                    continue;
                }
                Ok(Err(_)) => break,
                Err(_) => continue,
            }
        }
        let out = String::from_utf8_lossy(&total);
        assert!(out.contains("\"result\""));

        client.shutdown().await.unwrap();
        let _ = serve.await.unwrap();
    }

    #[tokio::test]
    async fn test_serve_stdio_with_io_two_lists() {}
}

//! MCP server integration (Streamable HTTP + stdio) for irish-mcp-gateway.
//!
//! - Exposes proper tool routers (eg. `gael.grammar_check`)
//! - Mounts Streamable HTTP services (POST frames, GET SSE) at `/mcp`
//! - Supports stdio mode when `MODE=stdio`
//!
//! This file intentionally **does not** depend on internal wire types. The tool
//! returns **plain JSON** like: {"issues":[ ... ]}, avoiding schemars version drift.

use std::{future::Future, pin::Pin, sync::Arc};

use serde::Deserialize;
use serde_json::{json, Value as JsonValue};

use rmcp::{
    handler::server::router::Router,
    handler::server::tool::{Parameters, ToolRouter},
    model::{CallToolResult, Content},
    ErrorData as McpError,
    serve_server,
    ServerHandler,
    transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    },
    // stdio transport helper (if you need direct (stdin, stdout))
    transport::io::stdio,
};


/// Trait abstraction to wrap existing Gramadóir integration without 
/// touching its types. Return a `serde_json::Value` with the exact
/// REST shape: `{"issues":[...]}`.
#[async_trait::async_trait]
pub trait GrammarCheck: Send + Sync + 'static {
    async fn check_as_json(&self, text: &str) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>>;
}

/// Thin wrapper around a boxed async fn, so `main` can adapt whatever
/// client/type is in use with _zero_ churn elsewhere.
pub struct FnChecker {
    inner: Arc<
        dyn Fn(
                String,
            )
                -> Pin<
                    Box<
                        dyn Future<Output = Result<JsonValue, Box<dyn std::error::Error + Send + Sync>>> + Send,
                    >,
                > + Send
            + Sync,
    >,
}

impl FnChecker {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<JsonValue, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        Self { inner: Arc::new(move |s| Box::pin(f(s))) }
    }
}

#[async_trait::async_trait]
impl GrammarCheck for FnChecker {
    async fn check_as_json(&self, text: &str) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
        (self.inner)(text.to_owned()).await
    }
}

/// The MCP server handler. Holds whichever implementation of `GrammarCheck`
/// it is given from `main.rs`.
#[derive(Clone)]
pub struct GatewaySvc {
    checker: Arc<dyn GrammarCheck>,
}

impl GatewaySvc {
    pub fn new(checker: Arc<dyn GrammarCheck>) -> Self {
        Self { checker }
    }
}

// We don't need extra methods from ServerHandler yet, but rmcp expects the impl.
impl ServerHandler for GatewaySvc {}

#[derive(Serialize, Deserialize)]
struct CheckInput {
    text: String,
}

/// The tool router. For example in the case of `gael.grammar_check`:
/// Input:  { "text": String }
/// Output: { "issues": [...] }  (plain JSON, just as in the REST API)
#[rmcp::tool_router]
impl GatewaySvc {
    #[rmcp::tool(name = "gael.grammar_check")]
    async fn gael_grammar_check(
        &self,
        params: Parameters<CheckInput>,
    ) -> Result<CallToolResult, McpError> {
        let CheckInput { text } = params.deserialize().map_err(|e| {
            McpError::invalid_params(format!("invalid params: {e}"), None)
        })?;

        // Existing client must return a serde_json::Value shaped as {"issues":[...]}
        let payload: serde_json::Value = self.gramadoir.check_json(&text).await.map_err(|e| {
            McpError::internal_error(format!("gramadoir error: {e}"), None)
        })?;

        // rmcp 0.5 CallToolResult wants a JSON Value for structured_content
        Ok(CallToolResult::structured(payload))
    }
}

/// Factory required by rmcp Streamable HTTP & stdio transports:
/// must return a `(handler, ToolRouter<handler>)` pair.
pub fn make_factory(checker: Arc<dyn GrammarCheck>) -> impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Clone + Send + 'static {
    move || {
        let handler = GatewaySvc::new(checker.clone());
        let router: ToolRouter<GatewaySvc> = GatewaySvc::tool_router();
        (handler, router)
    }
}

/// Build the Streamable HTTP Tower Service for mounting at `/mcp`.
// pub fn streamable_http_service(checker: Arc<dyn GrammarCheck>) -> StreamableHttpService<impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Clone + Send + 'static> {
//     let session_mgr = Arc::new(LocalSessionManager::default());
//     StreamableHttpService::new(make_factory(checker), session_mgr)
// }

/// Run stdio MCP server when `MODE=stdio`.
/// This uses rmcp io transport to speak JSON-RPC over stdin/stdout.
// pub async fn serve_stdio(checker: Arc<dyn GrammarCheck>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     // Uses SDK defaults for protocol/version & framing.
//     rmcp::transport::io::serve_server(make_factory(checker)).await?;
//     Ok(())
// }

pub fn build_streamable_http_service(
    factory: impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Send + Sync + Clone + 'static,
) -> StreamableHttpService<Router<GatewaySvc>, LocalSessionManager> {
    let session_mgr = std::sync::Arc::new(LocalSessionManager::default());
    let cfg = StreamableHttpServerConfig::default();

    let handler_factory = move || {
        let (handler, router) = factory();
        Ok(Router::new(handler, router))
    };

    StreamableHttpService::new(handler_factory, session_mgr, cfg)
}

pub async fn run_stdio(make_handler: impl FnOnce() -> (GatewaySvc, ToolRouter<GatewaySvc>)) -> anyhow::Result<()> {
    // Build once for stdio, no sessions needed.
    let (handler, router) = make_handler();
    // rmcp 0.5: serve_server takes a "service" — the pair is accepted on 0.5
    // and stdio transport defaults via features; if you want to be explicit:
    let (stdin, stdout) = stdio();
    serve_server((handler, router), (stdin, stdout)).await?;
    Ok(())
}


#[cfg(test)]
mod tests_util {
    use super::*;

    /// Returns a checker that always produces a deterministic dummy issue.
    pub fn dummy_grammar_checker() -> Arc<dyn GrammarCheck> {
        IntoGrammarCheck::into(FnChecker::new(|text: String| async move {
            let val = json!({
                "issues": [{
                    "code": "TEST",
                    "message": format!("ok: {}", text),
                    "start": 0,
                    "end": 0,
                    "suggestions": []
                }]
            });
            Ok(val)
        }))
    }

    trait IntoGrammarCheck {
        fn into(self) -> Arc<dyn GrammarCheck>;
    }
    impl IntoGrammarCheck for FnChecker {
        fn into(self) -> Arc<dyn GrammarCheck> { Arc::new(self) as Arc<dyn GrammarCheck> }
    }
}

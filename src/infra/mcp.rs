//! MCP server integration (Streamable HTTP + stdio) for irish-mcp-gateway.
//!
//! - Exposes proper tool routers (eg. `gael.grammar_check`)
//! - Mounts Streamable HTTP services (POST frames, GET SSE) at `/mcp`
//! - Supports stdio mode when `MODE=stdio`
//!
//! This file intentionally **does not** depend on internal wire types. The tool
//! returns **plain JSON** like: {"issues":[ ... ]}, avoiding schemars version drift.

use std::{future::Future, pin::Pin, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use rmcp::{
    ErrorData as McpError,
    ServerHandler,
    model::{CallToolResult, Content, JsonObject},
    handler::server::{
        router::Router,
        tool::{Parameters, ToolRouter},
    },
    serve_server,
};

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpService, StreamableHttpServerConfig},
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
    #[rmcp::tool(name = "gael.grammar_check", description = "Run Gramadóir and return {\"issues\": [...]} exactly as JSON")]
    async fn gael_grammar_check(
        &self,
        params: Parameters<JsonObject>,
    ) -> Result<CallToolResult, McpError> {
        // Parameters<T> is a tuple struct in 0.5
        let text = params
            .0
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("missing required field: text", None))?
            .to_owned();

        // Use your existing field (it's named `checker`)
        let payload: JsonValue = self
            .checker
            .check_as_json(&text)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Return plain JSON content (no custom schema)
        let content = Content::json(payload)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![content]))
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
pub async fn serve_stdio(
    factory: impl FnOnce() -> (GatewaySvc, ToolRouter<GatewaySvc>),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (handler, router) = factory();
    let service = Router::new(handler).with_tools(router); // <-- Service<RoleServer>

    // use tokio stdio
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    serve_server(service, (stdin, stdout)).await?;
    Ok(())
}

pub fn make_streamable_http_service(
    factory: impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Send + Sync + Clone + 'static,
) -> StreamableHttpService<Router<GatewaySvc>, LocalSessionManager> {
    let session_mgr = Arc::new(LocalSessionManager::default());
    let cfg = StreamableHttpServerConfig::default();

    let service_factory = move || {
        let (handler, tools) = factory();
        let service = Router::new(handler).with_tools(tools);
        Ok(service)
    };

    StreamableHttpService::new(service_factory, session_mgr, cfg)
}

// pub async fn run_stdio(make_handler: impl FnOnce() -> (GatewaySvc, ToolRouter<GatewaySvc>)) -> anyhow::Result<()> {
//     // Build once for stdio, no sessions needed.
//     let (handler, router) = make_handler();
//     // rmcp 0.5: serve_server takes a "service" — the pair is accepted on 0.5
//     // and stdio transport defaults via features; if you want to be explicit:
//     let (stdin, stdout) = stdio();
//     serve_server((handler, router), (stdin, stdout)).await?;
//     Ok(())
// }


#[cfg(test)]
mod tests_util {
    use super::*;
    use serde_json::json;

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

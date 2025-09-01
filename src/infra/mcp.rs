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

use rmcp::content::Content;
use rmcp::handler::server::{
    tool::{CallToolResult, Parameters, ToolError},
    ServerHandler,
};
use rmcp::router::ToolRouter;
use rmcp::transport::streamable_http_server::{
    service::StreamableHttpService,
    session::local::LocalSessionManager,
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

#[derive(Deserialize)]
struct CheckInput {
    text: String,
}

/// The tool router. For example in the case of `gael.grammar_check`:
/// Input:  { "text": String }
/// Output: { "issues": [...] }  (plain JSON, just as in the REST API)
#[rmcp::tool_router]
impl GatewaySvc {
    #[rmcp::tool(name = "gael.grammar_check")]
    async fn gael_grammar_check(&self, params: Parameters<P>) -> CallToolResult {
        let CheckInput { text } = params.deserialize::<CheckInput>()
            .map_err(|e| ToolError::InvalidParams { message: e.to_string() })?;

        // Delegate to the existing tool through the adapter; return *plain JSON*
        // like {"issues":[...]} so we never expose internal wire types.
        let payload = self.checker
            .check_as_json(&text)
            .await
            .map_err(|e| ToolError::InternalError { message: e.to_string() })?;

        // MCP result as structured content containing JSON
        let content = Content::from(rmcp::Json(payload));
        Ok(rmcp::handler::server::tool::CallToolResponse {
            structured_content: Some(vec![content]),
            ..Default::default()
        })
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
pub fn streamable_http_service(checker: Arc<dyn GrammarCheck>) -> StreamableHttpService<impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Clone + Send + 'static> {
    let session_mgr = Arc::new(LocalSessionManager::default());
    StreamableHttpService::new(make_factory(checker), session_mgr)
}

/// Run stdio MCP server when `MODE=stdio`.
/// This uses rmcp io transport to speak JSON-RPC over stdin/stdout.
pub async fn serve_stdio(checker: Arc<dyn GrammarCheck>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Uses SDK defaults for protocol/version & framing.
    rmcp::transport::io::serve_server(make_factory(checker)).await?;
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

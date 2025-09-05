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
    model::JsonObject,
    handler::server::{
        router::Router,
        tool::{Parameters, ToolRouter},
    },
    serve_server,
};

use rmcp::transport::streamable_http_server::{
    tower::{StreamableHttpService, StreamableHttpServerConfig},
};

pub use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;

use crate::clients::gramadoir::GramadoirRemote;

/// Trait abstraction to wrap existing Gramadóir integration without 
/// touching its types. Return a `serde_json::Value` with the exact
/// REST shape: `{"issues":[... ]}`.
#[async_trait::async_trait]
pub trait GrammarCheck: Send + Sync + 'static {
    async fn check_as_json(&self, text: &str) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>>;
}

/// Thin wrapper around a boxed async fn, so `main` can adapt whatever
/// client/type is in use with _zero_ churn elsewhere.
type JsonError = Box<dyn std::error::Error + Send + Sync>;
type JsonFuture = Pin<Box<dyn Future<Output = Result<JsonValue, JsonError>> + Send>>;
pub struct FnChecker {
    inner: Arc<dyn Fn(String) -> JsonFuture + Send + Sync>,
}

impl FnChecker {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<JsonValue, JsonError>> + Send + 'static,
    {
        Self { inner: Arc::new(move |s| Box::pin(f(s))) }
    }
}

#[async_trait::async_trait]
impl GrammarCheck for FnChecker {
    async fn check_as_json(&self, text: &str) -> Result<JsonValue, JsonError> {
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
    #[allow(dead_code)]
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
    ) -> Result<rmcp::Json<serde_json::Value>, McpError> {
        tracing::debug!(params = ?params.0, "gael_grammar_check invoked");
        let text = params
            .0
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("missing required field: text", None))?
            .to_owned();

        let payload = self
            .checker
            .check_as_json(&text)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        tracing::trace!(payload = %payload, "gael_grammar_check returning payload");

        // ✅ Spec-compliant: goes to `structuredContent`
        Ok(rmcp::Json(payload))
    }
}

/// Factory required by rmcp Streamable HTTP & stdio transports:
/// must return a `(handler, ToolRouter<handler>)` pair.
#[allow(dead_code)]
pub fn make_factory(checker: Arc<dyn GrammarCheck>) -> impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Clone + Send + 'static {
    move || {
        let handler = GatewaySvc::new(checker.clone());
        let router: ToolRouter<GatewaySvc> = GatewaySvc::tool_router();
        (handler, router)
    }
}

/// Run stdio MCP server when `MODE=stdio`.
/// This uses rmcp io transport to speak JSON-RPC over stdin/stdout.
#[allow(dead_code)]
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

pub async fn serve_stdio_from(
    factory: impl FnOnce() -> (GatewaySvc, ToolRouter<GatewaySvc>),
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (handler, tools) = factory();
    let service = Router::new(handler).with_tools(tools);
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    serve_server(service, (stdin, stdout)).await?;
    Ok(())
}

pub fn make_streamable_http_service(
    factory: impl Fn() -> (GatewaySvc, ToolRouter<GatewaySvc>) + Send + Sync + Clone + 'static,
    session_mgr: Arc<LocalSessionManager>,
) -> StreamableHttpService<GatewayRouter, LocalSessionManager> {
    tracing::info!("make_streamable_http_service invoked");
    let cfg = StreamableHttpServerConfig::default();
    tracing::debug!(stateful_mode = %cfg.stateful_mode, keep_alive = ?cfg.sse_keep_alive, "StreamableHttpServerConfig");

    let service_factory = move || {
        let (handler, tools) = factory();
        tracing::debug!("service_factory invoked: building Router with tools");
        let service = Router::new(handler).with_tools(tools);
        Ok(service)
    };

    let svc = StreamableHttpService::new(service_factory, session_mgr.clone(), cfg);
    tracing::info!(session_mgr_ptr = ?(&*session_mgr as *const _), "StreamableHttpService created");
    svc
}

pub type GatewayRouter = Router<GatewaySvc>;

pub fn factory_with_checker(
    checker: Arc<dyn GrammarCheck + Send + Sync>,
) -> (GatewaySvc, ToolRouter<GatewaySvc>) {
    let handler = GatewaySvc { checker };
    let tools = GatewaySvc::tool_router(); // <— provided by #[rmcp::tool_router]
    (handler, tools)
}

pub fn factory_from_env() -> (GatewaySvc, ToolRouter<GatewaySvc>) {
    match std::env::var("GRAMADOIR_BASE_URL") {
        Ok(base) if !base.trim().is_empty() => {
            let checker = Arc::new(GramadoirRemote::new(base)) as Arc<dyn GrammarCheck + Send + Sync>;
            factory_with_checker(checker)
        }
        _ => {
            // Provide a checker that returns a clear error so the service stays up
            // but clients get actionable feedback until configured.
            let checker = FnChecker::new(|_text: String| async move {
                Err::<serde_json::Value, JsonError>(
                    std::io::Error::other(
                        "GRAMADOIR_BASE_URL not configured; set it to enable gael.grammar_check",
                    )
                    .into(),
                )
            });
            factory_with_checker(Arc::new(checker) as Arc<dyn GrammarCheck + Send + Sync>)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value as JsonValue};

    /// A tiny helper to build a checker that returns a fixed JSON payload.
    fn dummy_checker() -> Arc<dyn GrammarCheck> {
        IntoGrammarCheck::into(FnChecker::new(|text: String| async move {
            // reflect input to prove we got the value through the path
            Ok(json!({
                "issues": [{
                    "code": "TEST",
                    "message": format!("ok: {}", text),
                    "start": 0, "end": 0, "suggestions": []
                }]
            }))
        }))
    }

    trait IntoGrammarCheck {
        fn into(self) -> Arc<dyn GrammarCheck>;
    }
    impl IntoGrammarCheck for FnChecker {
        fn into(self) -> Arc<dyn GrammarCheck> { Arc::new(self) as Arc<dyn GrammarCheck> }
    }

    #[tokio::test]
    async fn tool_call_success_returns_plain_json_issues() {
        let svc = GatewaySvc::new(dummy_checker());
        let mut obj = JsonObject::new();
        obj.insert("text".to_string(), JsonValue::String("Tá an peann ar an mbord".into()));

        // Method now returns rmcp::Json<serde_json::Value>
        let rmcp::Json(val) = svc.gael_grammar_check(Parameters(obj)).await.expect("tool should succeed");

        let issues = val["issues"].as_array().expect("issues array");
        assert!(!issues.is_empty(), "expected at least one dummy issue");
        assert_eq!(issues[0]["code"], "TEST");
        assert!(issues[0]["message"].as_str().unwrap().starts_with("ok: Tá an peann"));
    }

    #[tokio::test]
    async fn tool_call_missing_text_is_invalid_params() {
        let svc = GatewaySvc::new(dummy_checker());
        let obj = JsonObject::new(); // no "text"

        let res = svc.gael_grammar_check(Parameters(obj)).await;

        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("expected invalid params error, got Ok"),
        };

        // JSON-RPC invalid params is -32602
        assert_eq!(err.code.0, -32602, "expected invalid params code");
        assert!(
            err.message.contains("missing required field: text"),
            "message should mention missing text, got: {}",
            err.message
        );
    }

    #[test]
    fn tool_router_contains_gael_grammar_check() {
        let router: ToolRouter<GatewaySvc> = GatewaySvc::tool_router();
        // ToolRouter implements IntoIterator over routes; route.name() yields &str.
        let names: Vec<String> = router.into_iter().map(|r| r.name().to_string()).collect();
        assert!(names.iter().any(|n| n == "gael.grammar_check"), "missing tool 'gael.grammar_check', got: {:?}", names);
    }

    #[test]
    fn streamable_http_service_builds() {
        // Just a construction smoke test for the Streamable HTTP service.
        // No network I/O, just ensures the factory produces a Service<RoleServer>.
        let checker = dummy_checker();
        let factory = move || {
            let handler = GatewaySvc::new(checker.clone());
            let tools: ToolRouter<GatewaySvc> = GatewaySvc::tool_router();
            (handler, tools)
        };

        let session_mgr = Arc::new(LocalSessionManager::default());
        let _svc = crate::infra::mcp::make_streamable_http_service(factory, session_mgr);
        // If we got here, type constraints & factory shape are satisfied.
    }

    #[cfg(test)]
    pub fn test_factory_with_checker(
        checker: Arc<dyn GrammarCheck + Send + Sync>,
    ) -> (GatewaySvc, ToolRouter<GatewaySvc>) {
        factory_with_checker(checker)
    }
}

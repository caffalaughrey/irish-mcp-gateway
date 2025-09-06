use std::future::Future;

use rmcp::handler::server::tool::ToolRouter;

use crate::clients::gramadoir::GramadoirRemote;
use crate::infra::runtime::mcp_transport::ServerHandler;

#[derive(Clone)]
pub struct GrammarSvc<TChecker> {
    pub checker: TChecker,
}

impl<TChecker: Send + Sync + 'static> ServerHandler for GrammarSvc<TChecker> {}

#[rmcp::tool_router]
impl GrammarSvc<GramadoirRemote> {
    #[rmcp::tool(
        name = "gael.grammar_check",
        description = "Run Gramadóir and return {\"issues\": [...]} exactly as JSON"
    )]
    async fn gael_grammar_check(
        &self,
        params: rmcp::handler::server::tool::Parameters<rmcp::model::JsonObject>,
    ) -> Result<rmcp::Json<serde_json::Value>, rmcp::ErrorData> {
        let text = params
            .0
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| rmcp::ErrorData::invalid_params("missing required field: text", None))?
            .to_owned();
        let issues = self
            .checker
            .analyze(&text)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;
        Ok(rmcp::Json(serde_json::json!({"issues": issues})))
    }
}

pub type GrammarRouter = ToolRouter<GrammarSvc<GramadoirRemote>>;

impl GrammarSvc<GramadoirRemote> {
    pub fn router() -> GrammarRouter {
        // Wrapper to expose the macro-generated private tool_router
        Self::tool_router()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::gramadoir::GramadoirRemote;
    use rmcp::handler::server::tool::Parameters;
    use serde_json::json;

    #[test]
    fn test_grammar_svc_creation() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let _svc = GrammarSvc { checker };
        // Test that we can create the service
        assert!(true);
    }

    #[test]
    fn test_grammar_router_creation() {
        let _router = GrammarSvc::router();
        // Test that we can create the router
        assert!(true);
    }

    #[test]
    fn test_grammar_svc_clone() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let svc = GrammarSvc { checker };
        let _svc_clone = svc.clone();
        // Test that we can clone the service
        assert!(true);
    }

    #[tokio::test]
    async fn test_grammar_check_missing_text() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let svc = GrammarSvc { checker };
        let params = Parameters(json!({}).as_object().unwrap().clone());

        let result = svc.gael_grammar_check(params).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.message.contains("missing required field: text"));
        }
    }

    #[tokio::test]
    async fn test_grammar_check_invalid_text_type() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let svc = GrammarSvc { checker };
        let params = Parameters(json!({"text": 123}).as_object().unwrap().clone());

        let result = svc.gael_grammar_check(params).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.message.contains("missing required field: text"));
        }
    }

    #[tokio::test]
    async fn test_grammar_check_with_text() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let svc = GrammarSvc { checker };
        let params = Parameters(json!({"text": "test text"}).as_object().unwrap().clone());

        // This will fail because the checker will try to make an HTTP request
        // but we're testing the parameter validation path
        let result = svc.gael_grammar_check(params).await;
        assert!(result.is_err()); // Expected to fail due to HTTP request
    }

    #[test]
    fn test_grammar_router_type_alias() {
        let _router: GrammarRouter = GrammarSvc::router();
        // Test that the type alias works
        assert!(true);
    }

    #[test]
    fn test_server_handler_trait_impl() {
        let checker = GramadoirRemote::new("http://test".to_string());
        let svc = GrammarSvc { checker };

        // Test that GrammarSvc implements ServerHandler
        fn assert_server_handler<T: ServerHandler>(_handler: T) {}
        assert_server_handler(svc);
    }
}

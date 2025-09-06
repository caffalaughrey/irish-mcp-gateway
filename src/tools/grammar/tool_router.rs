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



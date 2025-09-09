use std::future::Future;
use rmcp::handler::server::tool::{Parameters, ToolRouter};

use crate::infra::runtime::mcp_transport::ServerHandler;

#[derive(Clone)]
pub struct UnifiedSvc;

impl ServerHandler for UnifiedSvc {}

#[rmcp::tool_router]
impl UnifiedSvc {
    #[rmcp::tool(name = "gael.grammar_check", description = "Irish grammar via Gramadóir")]
    async fn grammar(
        &self,
        params: Parameters<rmcp::model::JsonObject>,
    ) -> Result<rmcp::Json<serde_json::Value>, rmcp::ErrorData> {
        let text = params
            .0
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| rmcp::ErrorData::invalid_params("missing required field: text", None))?
            .to_owned();
        let base = std::env::var("GRAMADOIR_BASE_URL").unwrap_or_default();
        let client = crate::clients::gramadoir::GramadoirRemote::new(base);
        let issues = client
            .analyze(&text)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;
        Ok(rmcp::Json(serde_json::json!({"issues": issues})))
    }

    #[rmcp::tool(name = "spell.check", description = "Irish spellcheck via GaelSpell")]
    async fn spell(
        &self,
        params: Parameters<rmcp::model::JsonObject>,
    ) -> Result<rmcp::Json<serde_json::Value>, rmcp::ErrorData> {
        let text = params
            .0
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| rmcp::ErrorData::invalid_params("missing required field: text", None))?
            .to_owned();
        let base = std::env::var("SPELLCHECK_BASE_URL").unwrap_or_default();
        let client = crate::clients::gaelspell::GaelspellRemote::new(base);
        let corrections = client
            .check(&text)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e, None))?;
        Ok(rmcp::Json(serde_json::json!({"corrections": corrections})))
    }
}

pub type UnifiedRouter = ToolRouter<UnifiedSvc>;

impl UnifiedSvc {
    pub fn router() -> UnifiedRouter {
        Self::tool_router()
    }
}



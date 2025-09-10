use std::future::Future;
use rmcp::handler::server::tool::{Parameters, ToolRouter};

use crate::infra::runtime::mcp_transport::ServerHandler;
use crate::infra::config::AppConfig;

#[derive(Clone)]
pub struct UnifiedSvc;

impl ServerHandler for UnifiedSvc {}

#[rmcp::tool_router]
impl UnifiedSvc {
    #[rmcp::tool(name = "grammar.check", description = "Irish grammar via Gramadóir")]
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
        let app_cfg = AppConfig::from_env_and_toml();
        let client = crate::clients::gramadoir::GramadoirRemote::from_config(&app_cfg.grammar);
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
        let app_cfg = AppConfig::from_env_and_toml();
        let client = crate::clients::gaelspell::GaelspellRemote::from_config(&app_cfg.spell);
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



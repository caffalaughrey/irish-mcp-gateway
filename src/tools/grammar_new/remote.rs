use async_trait::async_trait;
use serde_json::json;

use crate::core::tool::{Tool, ToolSpec};

#[derive(Clone)]
pub struct GrammarRemoteBackend {
    pub(crate) base_url: String,
}

impl GrammarRemoteBackend {
    pub fn new(base_url: impl Into<String>) -> Self { Self { base_url: base_url.into() } }
}

impl ToolSpec for GrammarRemoteBackend {
    fn name(&self) -> &'static str { "gael.grammar_check.v2" }
    fn description(&self) -> &'static str { "Irish grammar via Gramadóir (remote backend)" }
    fn input_schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"]})
    }
}

#[async_trait]
impl Tool for GrammarRemoteBackend {
    async fn call(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let text = args.get("text").and_then(|v| v.as_str()).ok_or("missing 'text'")?;
        // Delegate to existing client to avoid duplication
        let cli = crate::clients::gramadoir::GramadoirRemote::new(self.base_url.clone());
        let issues = cli.analyze(text).await.map_err(|e| e.to_string())?;
        Ok(json!({"issues": issues}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn remote_backend_calls_and_shapes() {
        let server = MockServer::start();
        server.mock(|when, then|{
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .json_body(json!({"teacs":"Slán"}));
            then.status(200).json_body(json!([{
                "context":"Slán","contextoffset":"0","errorlength":"1","fromx":"0","fromy":"0","msg":"Spell","ruleId":"SPELL","tox":"1","toy":"0"
            }]));
        });
        let tool = GrammarRemoteBackend::new(server.base_url());
        let out = tool.call(&json!({"text":"Slán"})).await.unwrap();
        assert!(out["issues"].is_array());
    }
}



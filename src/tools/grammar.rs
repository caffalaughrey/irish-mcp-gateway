use async_trait::async_trait;
use serde_json::json;

use crate::clients::gramadoir::GramadoirRemote;
use crate::domain::{Tool, ToolError};

#[derive(Clone)]
pub struct GrammarTool {
    client: GramadoirRemote,
}

impl GrammarTool {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: GramadoirRemote::new(base_url),
        }
    }
}

#[async_trait]
impl Tool for GrammarTool {
    fn name(&self) -> &'static str {
        "gael.grammar_check"
    }
    fn description(&self) -> &'static str {
        "Irish grammar/spell check via Gramadóir (remote)"
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
          "type":"object",
          "properties": { "text": { "type":"string" } },
          "required": ["text"]
        })
    }
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let Some(text) = arguments.get("text").and_then(|v| v.as_str()) else {
            return Err(ToolError::Message("missing 'text'".into()));
        };
        let issues = self
            .client
            .analyze(text)
            .await
            .map_err(ToolError::Message)?;
        Ok(json!({ "issues": issues }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn it_calls_remote_and_shapes_output() {
        let server = MockServer::start();

        server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .json_body(json!({"teacs":"Tá sé go maith"}));
            then.status(200).json_body(json!([{
                "context":"Tá sé go maith",
                "contextoffset":"0",
                "errorlength":"2",
                "fromx":"0",
                "fromy":"0",
                "msg":"Spelling",
                "ruleId":"SPELL",
                "tox":"2",
                "toy":"0"
            }]));
        });

        let tool = GrammarTool::new(server.base_url());
        let out = tool.call(&json!({"text":"Tá sé go maith"})).await.unwrap();
        assert!(out["issues"].is_array());
        assert_eq!(out["issues"][0]["code"], "SPELL");
    }

    #[tokio::test]
    async fn it_validates_missing_text() {
        let tool = GrammarTool::new("http://localhost:0");
        let err = tool.call(&json!({})).await.unwrap_err();
        assert!(err.to_string().contains("missing 'text'"));
    }
}

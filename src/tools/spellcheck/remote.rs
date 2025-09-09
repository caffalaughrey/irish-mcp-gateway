use async_trait::async_trait;

use crate::core::tool::{Tool, ToolSpec};
use crate::clients::gaelspell::GaelspellRemote;
 

#[derive(Clone)]
pub struct SpellcheckRemoteBackend {
    client: GaelspellRemote,
}

impl SpellcheckRemoteBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { client: GaelspellRemote::new(base_url) }
    }

    #[allow(dead_code)]
    pub async fn health(&self) -> bool { self.client.health().await }
}

impl ToolSpec for SpellcheckRemoteBackend {
    fn name(&self) -> &'static str {
        "spell.check"
    }
    fn description(&self) -> &'static str {
        "Irish spellcheck (remote backend placeholder)"
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"]})
    }
}

#[async_trait]
impl Tool for SpellcheckRemoteBackend {
    async fn call(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("missing 'text'")?;
        let corrections = self.client.check(text).await?;
        Ok(serde_json::json!({"corrections": corrections}))
    }

    async fn health(&self) -> bool {
        SpellcheckRemoteBackend::health(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_backend_returns_empty_on_happy_path() {
        use httpmock::prelude::*;
        use serde_json::json;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST)
                .path("/api/gaelspell/1.0")
                .json_body(json!({"teacs":"Dia"}));
            then.status(200).json_body(serde_json::json!([]));
        });
        let tool = SpellcheckRemoteBackend::new(server.base_url());
        let out = tool.call(&serde_json::json!({"text":"Dia"})).await.unwrap();
        assert!(out["corrections"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn remote_backend_returns_error_on_missing_text() {
        let tool = SpellcheckRemoteBackend::new("http://example");
        let err = tool.call(&serde_json::json!({})).await.unwrap_err();
        assert!(err.contains("missing 'text'"));
    }

    #[tokio::test]
    async fn remote_backend_returns_error_on_invalid_text_type() {
        let tool = SpellcheckRemoteBackend::new("http://example");
        let err = tool
            .call(&serde_json::json!({"text": 123}))
            .await
            .unwrap_err();
        assert!(err.contains("missing 'text'"));
    }

    #[test]
    fn tool_spec_metadata_present() {
        let t = SpellcheckRemoteBackend::new("http://example");
        assert_eq!(t.name(), "spell.check");
        assert!(t.description().contains("spellcheck"));
        let s = t.input_schema();
        assert_eq!(s["type"], "object");
    }
}

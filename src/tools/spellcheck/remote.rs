use async_trait::async_trait;

use crate::core::tool::{Tool, ToolSpec};
use crate::infra::runtime::limits::make_http_client;
use crate::infra::http::headers::generate_request_id;

#[derive(Clone)]
pub struct SpellcheckRemoteBackend {
    #[allow(dead_code)]
    pub(crate) base_url: String,
    http: reqwest::Client,
}

impl SpellcheckRemoteBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: make_http_client(),
        }
    }

    #[allow(dead_code)]
    pub async fn health(&self) -> bool {
        let id = generate_request_id();
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        match self
            .http
            .get(url)
            .header("x-request-id", id)
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

impl ToolSpec for SpellcheckRemoteBackend {
    fn name(&self) -> &'static str {
        "gael.spellcheck.v1"
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
        let _ = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("missing 'text'")?;
        // Placeholder: just echo empty corrections for now
        Ok(serde_json::json!({"corrections": []}))
    }

    async fn health(&self) -> bool {
        // Delegate to inherent health probe
        SpellcheckRemoteBackend::health(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_backend_returns_empty_on_happy_path() {
        let tool = SpellcheckRemoteBackend::new("http://example");
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
        assert_eq!(t.name(), "gael.spellcheck.v1");
        assert!(t.description().contains("spellcheck"));
        let s = t.input_schema();
        assert_eq!(s["type"], "object");
    }
}

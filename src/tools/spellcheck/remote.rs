use async_trait::async_trait;

use crate::core::tool::{Tool, ToolSpec};

#[derive(Clone)]
pub struct SpellcheckRemoteBackend {
    pub(crate) base_url: String,
}

impl SpellcheckRemoteBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_backend_placeholder_returns_empty() {
        let tool = SpellcheckRemoteBackend::new("http://example");
        let out = tool.call(&serde_json::json!({"text":"Dia"})).await.unwrap();
        assert!(out["corrections"].as_array().unwrap().is_empty());
    }
}

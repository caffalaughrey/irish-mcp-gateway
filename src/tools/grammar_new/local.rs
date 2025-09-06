use async_trait::async_trait;

use crate::core::tool::{Tool, ToolSpec};

#[derive(Clone, Default)]
pub struct GrammarLocalBackend;

impl ToolSpec for GrammarLocalBackend {
    fn name(&self) -> &'static str { "gael.grammar_check.v2" }
    fn description(&self) -> &'static str { "Irish grammar (local stub backend)" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"]})
    }
}

#[async_trait]
impl Tool for GrammarLocalBackend {
    async fn call(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let _ = args.get("text").and_then(|v| v.as_str()).ok_or("missing 'text'")?;
        // For now, local backend is a stub and returns an empty issues list
        Ok(serde_json::json!({"issues": []}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_backend_stub_returns_empty_issues() {
        let tool = GrammarLocalBackend::default();
        let out = tool.call(&serde_json::json!({"text":"abc"})).await.unwrap();
        assert!(out["issues"].as_array().unwrap().is_empty());
    }
}



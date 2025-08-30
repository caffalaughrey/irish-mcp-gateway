use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarIssue {
    pub code: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub suggestions: Vec<String>,
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> serde_json::Value;
    async fn call(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, to_value};

    #[test]
    fn it_knows_tool_error_display() {
        let e = ToolError::Message("boom".into());
        assert_eq!(e.to_string(), "boom");
    }

    #[test]
    fn it_knows_grammar_issue_serde_roundtrip() {
        let gi = GrammarIssue {
            code: "AGR".into(),
            message: "Agreement issue".into(),
            start: 1, end: 3, suggestions: vec!["X".into()]
        };
        let v = to_value(&gi).unwrap();
        let back: GrammarIssue = from_value(v).unwrap();
        assert_eq!(back.code, "AGR");
        assert_eq!(back.suggestions, vec!["X"]);
    }
}

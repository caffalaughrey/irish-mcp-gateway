use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarIssue {
    pub code: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub suggestions: Vec<String>,
}

// Legacy Tool trait removed - using core::tool::Tool instead

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, to_value};

    // ToolError test removed - using core::error::GatewayError instead

    #[test]
    fn it_knows_grammar_issue_serde_roundtrip() {
        let gi = GrammarIssue {
            code: "AGR".into(),
            message: "Agreement issue".into(),
            start: 1,
            end: 3,
            suggestions: vec!["X".into()],
        };
        let v = to_value(&gi).unwrap();
        let back: GrammarIssue = from_value(v).unwrap();
        assert_eq!(back.code, "AGR");
        assert_eq!(back.suggestions, vec!["X"]);
    }
}

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::GrammarIssue;
use crate::infra::mcp::GrammarCheck;

#[derive(Clone)]
pub struct GramadoirRemote {
    base: String,
    http: Client,
}

impl GramadoirRemote {
    pub fn new(base: impl Into<String>) -> Self {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(6))
            .build()
            .expect("reqwest client");
        Self { base: base.into(), http }
    }

    pub async fn analyze(&self, text: &str) -> Result<Vec<GrammarIssue>, String> {
        let url = format!("{}/api/gramadoir/1.0", self.base.trim_end_matches('/'));
        let resp = self.http
            .post(url)
            .json(&TeacsReq { teacs: text })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("upstream status {}", resp.status()));
        }
        // Upstream returns a top-level array
        let body: Vec<IssueWire> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body.into_iter().map(GrammarIssue::from).collect())
    }
}

#[async_trait::async_trait]
impl crate::infra::mcp::GrammarCheck for GramadoirRemote {
    async fn check_as_json(
        &self,
        text: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Reuse existing typed call and wrap it as JSON for MCP.
        let issues = self
            .analyze(text)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(serde_json::json!({ "issues": issues }))
    }
}


#[derive(Serialize, Deserialize)]
struct TeacsReq<'a> { teacs: &'a str }

#[derive(Serialize, Deserialize)]
struct IssueWire {
    // Sample fields seen upstream (strings, sometimes numbers-as-strings)
    context: Option<String>,
    #[serde(default)] contextoffset: String,
    #[serde(default)] errorlength: String,
    #[serde(default)] fromx: String,
    #[serde(default)] fromy: String,
    msg: String,
    #[serde(rename = "ruleId")]
    rule_id: String,
    #[serde(default)] tox: String,
    #[serde(default)] toy: String,
}

impl From<IssueWire> for GrammarIssue {
    fn from(w: IssueWire) -> Self {
        fn parse_usize(s: &str) -> usize { s.parse::<usize>().unwrap_or(0) }
        let start = parse_usize(&w.fromx);
        let end = {
            let tox = parse_usize(&w.tox);
            if tox > 0 { tox } else { start + parse_usize(&w.errorlength) }
        };
        GrammarIssue {
            code: w.rule_id,
            message: w.msg,
            start,
            end,
            suggestions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn it_maps_issues_from_remote_array() {
        let server = MockServer::start();

        // Expect POST /api/gramadoir/1.0 with {"teacs": "..."}
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .json_body_obj(&TeacsReq { teacs: "Tá an peann ar an bord" });
            then.status(200)
                .json_body(json!([IssueWire {
                    context: Some("Tá an peann ar an bord".into()),
                    contextoffset: "12".into(),
                    errorlength: "10".into(),
                    fromx: "12".into(),
                    fromy: "0".into(),
                    msg: "Initial mutation missing".into(),
                    rule_id: "Lingua::GA::Gramadoir/CLAOCHLU".into(),
                    tox: "21".into(),
                    toy: "0".into(),
                }]));
        });

        let cli = GramadoirRemote::new(server.base_url());
        let out = cli.analyze("Tá an peann ar an bord").await.unwrap();
        m.assert();

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "Lingua::GA::Gramadoir/CLAOCHLU");
        assert_eq!(out[0].message, "Initial mutation missing");
        assert_eq!(out[0].start, 12);
        assert_eq!(out[0].end, 21);
    }
}

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::GrammarIssue;
use crate::infra::runtime::limits::{make_http_client, retry_async};

#[derive(Clone)]
pub struct GramadoirRemote {
    base: String,
    http: Client,
}

impl GramadoirRemote {
    pub fn new(base: impl Into<String>) -> Self {
        let http = make_http_client();
        Self {
            base: base.into(),
            http,
        }
    }

    pub async fn analyze(&self, text: &str) -> Result<Vec<GrammarIssue>, String> {
        let url = format!("{}/api/gramadoir/1.0", self.base.trim_end_matches('/'));
        let http = self.http.clone();
        let url_clone = url.clone();
        tracing::debug!(endpoint = %url, "gramadoir.analyze request");
        let issues: Vec<IssueWire> = retry_async(2, move |_| {
            let http = http.clone();
            let url = url_clone.clone();
            let payload = TeacsReq { teacs: text };
            async move {
                let resp = http
                    .post(url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    if resp.status().is_server_error() {
                        return Err(format!("retryable status {}", resp.status()));
                    }
                    return Err(format!("upstream status {}", resp.status()));
                }
                resp.json::<Vec<IssueWire>>().await.map_err(|e| e.to_string())
            }
        })
        .await?;

        Ok(issues.into_iter().map(GrammarIssue::from).collect())
    }
}

#[async_trait::async_trait]
impl crate::infra::mcp::GrammarCheck for GramadoirRemote {
    async fn check_as_json(
        &self,
        text: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Reuse existing typed call and wrap it as JSON for MCP.
        let issues = self.analyze(text).await.map_err(std::io::Error::other)?;
        Ok(serde_json::json!({ "issues": issues }))
    }
}

#[derive(Serialize, Deserialize)]
struct TeacsReq<'a> {
    teacs: &'a str,
}

#[derive(Serialize, Deserialize)]
struct IssueWire {
    // Sample fields seen upstream (strings, sometimes numbers-as-strings)
    context: Option<String>,
    #[serde(default)]
    contextoffset: String,
    #[serde(default)]
    errorlength: String,
    #[serde(default)]
    fromx: String,
    #[serde(default)]
    fromy: String,
    msg: String,
    #[serde(rename = "ruleId")]
    rule_id: String,
    #[serde(default)]
    tox: String,
    #[serde(default)]
    toy: String,
}

impl From<IssueWire> for GrammarIssue {
    fn from(w: IssueWire) -> Self {
        fn parse_usize(s: &str) -> usize {
            s.parse::<usize>().unwrap_or(0)
        }
        let start = parse_usize(&w.fromx);
        let end = {
            let tox = parse_usize(&w.tox);
            if tox > 0 {
                tox
            } else {
                start + parse_usize(&w.errorlength)
            }
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
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn it_maps_issues_from_remote_array() {
        let server = MockServer::start();

        // Expect POST /api/gramadoir/1.0 with {"teacs": "..."}
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .json_body_obj(&TeacsReq {
                    teacs: "Tá an peann ar an bord",
                });
            then.status(200).json_body(json!([IssueWire {
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

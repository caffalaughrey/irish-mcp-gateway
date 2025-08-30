use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::GrammarIssue;

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
        let url = format!("{}/api/gramadoir/1.0/check", self.base.trim_end_matches('/'));
        let resp = self.http
            .post(url)
            .json(&CheckReq { text })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("upstream status {}", resp.status()));
        }
        let body: CheckResp = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body.issues.into_iter().map(GrammarIssue::from).collect())
    }
}

#[derive(Serialize, Deserialize)]
struct CheckReq<'a> { text: &'a str }

#[derive(Serialize, Deserialize)]
struct CheckResp { #[serde(default)] issues: Vec<IssueWire> }

#[derive(Serialize, Deserialize)]
struct IssueWire {
    code: String,
    message: String,
    start: usize,
    end: usize,
    #[serde(default)]
    suggestions: Vec<String>,
}

impl From<IssueWire> for GrammarIssue {
    fn from(w: IssueWire) -> Self {
        Self {
            code: w.code,
            message: w.message,
            start: w.start,
            end: w.end,
            suggestions: w.suggestions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn it_maps_issues_from_remote() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0/check")
                .json_body_obj(&CheckReq { text: "Dia daoibh" });
            then.status(200)
                .json_body_obj(&CheckResp {
                    issues: vec![IssueWire {
                        code: "AGR".into(),
                        message: "Agreement".into(),
                        start: 0,
                        end: 3,
                        suggestions: vec!["X".into()],
                    }],
                });
        });

        let cli = GramadoirRemote::new(server.base_url());
        let out = cli.analyze("Dia daoibh").await.unwrap();
        m.assert();

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "AGR");
        assert_eq!(out[0].suggestions, vec!["X"]);
    }
}

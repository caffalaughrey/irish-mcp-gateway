use reqwest::Client;
use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::domain::GrammarIssue;
use crate::infra::http::headers::{add_standard_headers, generate_request_id};
use crate::infra::runtime::limits::{make_http_client, make_http_client_with, retry_async};
use crate::infra::config::ToolConfig;

#[derive(Clone)]
pub struct GramadoirRemote {
    base: String,
    http: Client,
    retries: u32,
}

impl GramadoirRemote {
    pub fn new(base: impl Into<String>) -> Self {
        let http = make_http_client();
        Self {
            base: base.into(),
            http,
            retries: 2,
        }
    }

    pub fn from_config(cfg: &ToolConfig) -> Self {
        let base = cfg.base_url.clone().unwrap_or_else(|| "".to_string());
        let http = make_http_client_with(cfg);
        let retries = cfg.retries.unwrap_or(2);
        Self { base, http, retries }
    }

    #[allow(dead_code)]
    pub async fn health(&self) -> bool {
        let url = format!("{}/health", self.base.trim_end_matches('/'));
        let (builder, _rid) = add_standard_headers(self.http.get(url), None);
        match builder.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn analyze(&self, text: &str) -> Result<Vec<GrammarIssue>, String> {
        // TODO(refactor-fit-and-finish): Once we centralize ToolBackend HTTP clients,
        // thread a shared client and request-id middleware through this path.
        let url = format!("{}/api/gramadoir/1.0", self.base.trim_end_matches('/'));
        let http = self.http.clone();
        let url_clone = url.clone();
        tracing::debug!(endpoint = %url, "gramadoir.analyze request");
        let req_id = generate_request_id();
        let start = Instant::now();
        let attempts = self.retries;
        let res: Result<Vec<IssueWire>, String> = retry_async(attempts, move |_| {
            let http = http.clone();
            let url = url_clone.clone();
            let req_id = req_id.clone();
            let payload = TeacsReq { teacs: text };
            async move {
                let (builder, _rid) = add_standard_headers(http.post(url), Some(req_id));
                let resp = builder
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
                resp.json::<Vec<IssueWire>>()
                    .await
                    .map_err(|e| e.to_string())
            }
        })
        .await;
        if res.is_err() {
            crate::infra::logging::log_metric("grammar.check", "remote_error_total", 1.0);
        }
        let issues = res?;
        let elapsed_ms = start.elapsed().as_millis() as f64;
        crate::infra::logging::log_metric("grammar.check", "remote_latency_ms", elapsed_ms);
        Ok(issues.into_iter().map(GrammarIssue::from).collect())
    }
}

// Deprecated adapter removed: GramadoirRemote is used directly by the grammar tool router now.

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

    #[tokio::test]
    async fn it_retries_then_succeeds() {
        let server = MockServer::start();

        // First call 500
        server.mock(|when, then| {
            when.method(POST).path("/api/gramadoir/1.0");
            then.status(500).body("err");
        });

        // Second call 200 with empty array
        server.mock(|when, then| {
            when.method(POST).path("/api/gramadoir/1.0");
            then.status(200).json_body(json!([]));
        });

        let cli = GramadoirRemote::new(server.base_url());
        let out = cli.analyze("x").await.unwrap_or_default();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn it_returns_upstream_status_on_client_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/gramadoir/1.0");
            then.status(400).body("bad");
        });
        let cli = GramadoirRemote::new(server.base_url());
        let err = cli.analyze("x").await.unwrap_err();
        assert!(err.contains("upstream status"));
    }

    #[tokio::test]
    async fn it_sets_request_id_header() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/gramadoir/1.0")
                .header_exists("x-request-id")
                .header_exists("user-agent");
            then.status(200).json_body(json!([]));
        });
        let cli = GramadoirRemote::new(server.base_url());
        let _ = cli.analyze("x").await.unwrap();
        m.assert();
    }

    #[tokio::test]
    async fn health_gets_200() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(GET)
                .path("/health")
                .header_exists("x-request-id")
                .header_exists("user-agent");
            then.status(200).body("ok");
        });
        let cli = GramadoirRemote::new(server.base_url());
        assert!(cli.health().await);
        m.assert();
    }
}

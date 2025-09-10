use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::infra::http::headers::add_standard_headers;
use crate::infra::runtime::limits::{make_http_client, make_http_client_with, retry_async};
use crate::infra::config::ToolConfig;

#[derive(Clone)]
pub struct GaelspellRemote {
    base: String,
    http: Client,
    retries: u32,
}

impl GaelspellRemote {
    pub fn new(base: impl Into<String>) -> Self {
        let http = make_http_client();
        Self { base: base.into(), http, retries: 2 }
    }

    pub fn from_config(cfg: &ToolConfig) -> Self {
        let base = cfg
            .base_url
            .clone()
            .unwrap_or_else(|| "".to_string());
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

    pub async fn check(&self, text: &str) -> Result<Vec<Correction>, String> {
        let url = format!("{}/api/gaelspell/1.0", self.base.trim_end_matches('/'));
        let http = self.http.clone();
        let url_clone = url.clone();
        let payload = TeacsReq { teacs: text };

        let start = Instant::now();
        let attempts = self.retries;
        let res: Result<SpellWire, String> = retry_async(attempts, move |_| {
            let http = http.clone();
            let url = url_clone.clone();
            let payload = payload.clone();
            async move {
                let (builder, _rid) = add_standard_headers(http.post(url), None);
                let resp = builder.json(&payload).send().await.map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    if resp.status().is_server_error() {
                        return Err(format!("retryable status {}", resp.status()));
                    }
                    return Err(format!("upstream status {}", resp.status()));
                }
                resp.json::<SpellWire>().await.map_err(|e| e.to_string())
            }
        })
        .await;
        if res.is_err() {
            crate::infra::logging::log_metric("spell.check", "remote_error_total", 1.0);
        }
        let out = res?;
        let elapsed_ms = start.elapsed().as_millis() as f64;
        crate::infra::logging::log_metric("spell.check", "remote_latency_ms", elapsed_ms);
        Ok(out
            .into_iter()
            .map(|t| Correction::from(t))
            .collect())
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TeacsReq<'a> { teacs: &'a str }

#[derive(Serialize, Deserialize)]
struct TokenTupleWire(String, Vec<String>);

type SpellWire = Vec<TokenTupleWire>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Correction {
    pub token: String,
    pub start: usize,
    pub end: usize,
    pub suggestions: Vec<String>,
}

impl From<TokenTupleWire> for Correction {
    fn from(t: TokenTupleWire) -> Self {
        Self { token: t.0, start: 0, end: 0, suggestions: t.1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    #[tokio::test]
    async fn it_posts_to_gaelspell_and_maps_tokens() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/gaelspell/1.0")
                .json_body_obj(&TeacsReq { teacs: "Dia dhuit" });
            then.status(200).json_body(json!([ ["abcdef", ["abc","ab"]] ]));
        });

        let cli = GaelspellRemote::new(server.base_url());
        let out = cli.check("Dia dhuit").await.unwrap();
        m.assert();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].token, "abcdef");
        assert_eq!(out[0].suggestions[0], "abc");
    }
}



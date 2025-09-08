use reqwest::RequestBuilder;

/// Generate a simple request id suitable for logging/correlation.
pub fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("gw-{}-{}", now.as_secs(), now.subsec_nanos())
}

/// Add standard headers to an outgoing request. Returns the updated builder and the request id used.
pub fn add_standard_headers(
    builder: RequestBuilder,
    request_id: Option<String>,
) -> (RequestBuilder, String) {
    let rid = request_id.unwrap_or_else(generate_request_id);
    let b = builder.header("x-request-id", rid.as_str()).header(
        reqwest::header::USER_AGENT,
        format!("irish-mcp-gateway/{}", env!("CARGO_PKG_VERSION")),
    );
    (b, rid)
}

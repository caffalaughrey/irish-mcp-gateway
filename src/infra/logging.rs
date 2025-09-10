pub fn init() {
    // Initialize tracing subscriber once, honoring RUST_LOG if set.
    // Default to info level; allow override via RUST_LOG (e.g., "debug").
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();
}

/// Simple helper to log a metrics-like line until a real sink/exporter is added.
pub fn log_metric(tool: &str, metric: &str, value: f64) {
    tracing::info!(tool = tool, metric = metric, value = value, "metric");
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_is_idempotent() {
        super::init();
        super::init();
    }
}

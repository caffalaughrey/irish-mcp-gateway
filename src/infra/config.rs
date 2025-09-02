pub struct Config {
    pub mode: String, // "server" or "stdio"
    pub port: u16,
    pub deprecate_rest: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let mode = std::env::var("MODE").unwrap_or_else(|_| "http".into());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);
        let deprecate_rest = std::env::var("DEPRECATE_REST").map(|v| !v.is_empty()).unwrap_or(false);

        Self { mode, port, deprecate_rest }
    }
}

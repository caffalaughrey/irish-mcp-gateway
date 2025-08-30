pub struct Config {
    pub mode: String, // "server" | "stdio"
    pub port: u16,
}
impl Config {
    pub fn from_env() -> Self {
        let mode = std::env::var("MODE").unwrap_or_else(|_| "server".into());
        let port = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
        Self { mode, port }
    }
}

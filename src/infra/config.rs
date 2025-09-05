pub struct Config {
    pub mode: String, // "server" or "stdio"
    pub port: u16,
    pub deprecate_rest: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let mode = std::env::var("MODE").unwrap_or_else(|_| "server".into());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);
        let deprecate_rest = std::env::var("DEPRECATE_REST")
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        Self {
            mode,
            port,
            deprecate_rest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn defaults_to_server_8080_and_rest_enabled() {
        std::env::remove_var("MODE");
        std::env::remove_var("PORT");
        std::env::remove_var("DEPRECATE_REST");
        let cfg = Config::from_env();
        assert_eq!(cfg.mode, "server");
        assert_eq!(cfg.port, 8080);
        assert!(!cfg.deprecate_rest);
    }

    #[test]
    fn parses_env_overrides() {
        std::env::set_var("MODE", "stdio");
        std::env::set_var("PORT", "9090");
        std::env::set_var("DEPRECATE_REST", "1");
        let cfg = Config::from_env();
        assert_eq!(cfg.mode, "stdio");
        assert_eq!(cfg.port, 9090);
        assert!(cfg.deprecate_rest);
        std::env::remove_var("MODE");
        std::env::remove_var("PORT");
        std::env::remove_var("DEPRECATE_REST");
    }
}

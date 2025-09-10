use serde::Deserialize;

pub struct Config {
    pub mode: String, // "server" or "stdio"
    pub port: u16,
    pub deprecate_rest: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ToolConfig {
    pub base_url: Option<String>,
    pub request_timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub concurrency_limit: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct AppConfig {
    pub grammar: ToolConfig,
    pub spell: ToolConfig,
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

impl AppConfig {
    pub fn from_env_and_toml() -> Self {
        // Optional: load config file path from TOOLING_CONFIG; ignore errors.
        let file_cfg = std::env::var("TOOLING_CONFIG")
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str::<AppConfigToml>(&s).ok())
            .unwrap_or_default();

        let grammar = ToolConfig {
            base_url: std::env::var("GRAMADOIR_BASE_URL").ok().or(file_cfg.grammar.base_url),
            request_timeout_ms: std::env::var("GRAMMAR_TIMEOUT_MS").ok().and_then(|s| s.parse().ok()).or(file_cfg.grammar.request_timeout_ms),
            retries: std::env::var("GRAMMAR_RETRIES").ok().and_then(|s| s.parse().ok()).or(file_cfg.grammar.retries),
            concurrency_limit: std::env::var("GRAMMAR_CONCURRENCY").ok().and_then(|s| s.parse().ok()).or(file_cfg.grammar.concurrency_limit),
        };
        let spell = ToolConfig {
            base_url: std::env::var("SPELLCHECK_BASE_URL").ok().or(file_cfg.spell.base_url),
            request_timeout_ms: std::env::var("SPELL_TIMEOUT_MS").ok().and_then(|s| s.parse().ok()).or(file_cfg.spell.request_timeout_ms),
            retries: std::env::var("SPELL_RETRIES").ok().and_then(|s| s.parse().ok()).or(file_cfg.spell.retries),
            concurrency_limit: std::env::var("SPELL_CONCURRENCY").ok().and_then(|s| s.parse().ok()).or(file_cfg.spell.concurrency_limit),
        };

        AppConfig { grammar, spell }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ToolConfigToml {
    base_url: Option<String>,
    request_timeout_ms: Option<u64>,
    retries: Option<u32>,
    concurrency_limit: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AppConfigToml {
    grammar: ToolConfigToml,
    spell: ToolConfigToml,
}

#[cfg(test)]
mod tests {
    use super::Config;
    use serial_test::serial;

    #[test]
    #[serial]
    fn it_parses_env_and_defaults_serially() {
        // Defaults when unset/empty
        std::env::remove_var("MODE");
        std::env::set_var("PORT", "");
        std::env::set_var("DEPRECATE_REST", "");
        let cfg = Config::from_env();
        assert_eq!(cfg.mode, "server");
        assert_eq!(cfg.port, 8080);
        assert!(!cfg.deprecate_rest);

        // Overrides when provided
        std::env::set_var("MODE", "stdio");
        std::env::set_var("PORT", "9090");
        std::env::set_var("DEPRECATE_REST", "1");
        let cfg2 = Config::from_env();
        assert_eq!(cfg2.mode, "stdio");
        assert_eq!(cfg2.port, 9090);
        assert!(cfg2.deprecate_rest);

        // Cleanup
        std::env::remove_var("MODE");
        std::env::remove_var("PORT");
        std::env::remove_var("DEPRECATE_REST");
    }
}

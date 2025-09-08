use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "irish-mcp-gateway")]
#[command(about = "Irish MCP Gateway - Admin CLI")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Health check the service
    Health {
        /// Service URL to check
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,
    },
    /// Validate configuration
    Config {
        /// Validate config without starting service
        #[arg(long)]
        validate: bool,
    },
    /// Show service status and metrics
    Status {
        /// Service URL to check
        #[arg(short, long, default_value = "http://localhost:8080")]
        url: String,
    },
    /// Test grammar service connectivity
    TestGrammar {
        /// Grammar service URL
        #[arg(short, long)]
        url: Option<String>,
        /// Test text to check
        #[arg(short, long, default_value = "Tá an peann ar an mbord")]
        text: String,
    },
}

pub async fn run() -> ExitCode {
    let cli = Cli::parse();

    run_commands(cli.command).await
}

pub async fn run_commands(command: Commands) -> ExitCode {
    match command {
        Commands::Health { url } => match health_check(&url).await {
            Ok(_) => {
                println!("✅ Service is healthy");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Health check failed: {}", e);
                ExitCode::FAILURE
            }
        },
        Commands::Config { validate: _ } => match validate_config() {
            Ok(_) => {
                println!("✅ Configuration is valid");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Configuration validation failed: {}", e);
                ExitCode::FAILURE
            }
        },
        Commands::Status { url } => match show_status(&url).await {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("❌ Status check failed: {}", e);
                ExitCode::FAILURE
            }
        },
        Commands::TestGrammar { url, text } => match test_grammar(url, &text).await {
            Ok(_) => {
                println!("✅ Grammar service test passed");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Grammar service test failed: {}", e);
                ExitCode::FAILURE
            }
        },
    }
}

async fn health_check(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/healthz", url))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()).into())
    }
}

fn validate_config() -> Result<(), Box<dyn std::error::Error>> {
    let _config = crate::infra::config::Config::from_env();

    // Validate required environment variables
    let mode = std::env::var("MODE").unwrap_or_else(|_| "server".into());
    if !matches!(mode.as_str(), "server" | "stdio") {
        return Err(format!("Invalid MODE: {}. Must be 'server' or 'stdio'", mode).into());
    }

    if mode == "server" {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);

        if port == 0 {
            return Err("PORT cannot be 0".into());
        }
    }

    Ok(())
}

async fn show_status(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Health check
    let health_response = client
        .get(format!("{}/healthz", url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    println!(
        "🏥 Health Status: {}",
        if health_response.status().is_success() {
            "✅ Healthy"
        } else {
            "❌ Unhealthy"
        }
    );

    // Try to get tools list
    let tools_response = client
        .post(format!("{}/mcp", url))
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .timeout(std::time::Duration::from_millis(500))
        .send()
        .await;

    match tools_response {
        Ok(resp) if resp.status().is_success() => {
            println!("🔧 Tools: ✅ Available");
        }
        Ok(resp) => {
            println!("🔧 Tools: ❌ HTTP {}", resp.status());
        }
        Err(_) => {
            println!("🔧 Tools: ❌ Unavailable");
        }
    }

    // Configuration summary
    println!("\n📋 Configuration:");
    println!(
        "  Mode: {}",
        std::env::var("MODE").unwrap_or_else(|_| "server".into())
    );
    println!(
        "  Port: {}",
        std::env::var("PORT").unwrap_or_else(|_| "8080".into())
    );
    println!(
        "  Log Level: {}",
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
    );

    if let Ok(grammar_url) = std::env::var("GRAMADOIR_BASE_URL") {
        println!("  Grammar Service: {}", grammar_url);
    } else {
        println!("  Grammar Service: Not configured");
    }

    Ok(())
}

async fn test_grammar(url: Option<String>, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let grammar_url = url
        .or_else(|| std::env::var("GRAMADOIR_BASE_URL").ok())
        .ok_or("No grammar service URL provided")?;

    let client = crate::clients::gramadoir::GramadoirRemote::new(grammar_url);
    let issues = client.analyze(text).await?;

    println!("📝 Grammar check for: \"{}\"", text);
    println!("🔍 Found {} issues:", issues.len());

    for (i, issue) in issues.iter().enumerate() {
        println!(
            "  {}. {} ({}:{}:{})",
            i + 1,
            issue.message,
            issue.code,
            issue.start,
            issue.end
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use serial_test::serial;

    #[tokio::test]
    async fn test_health_check_success() {
        // This would need a running service; expect an error
        let result = health_check("http://localhost:9999").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn health_check_returns_ok_on_200() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/healthz");
            then.status(200).body("ok");
        });
        let ok = health_check(&server.base_url()).await;
        assert!(ok.is_ok());
    }

    #[test]
    #[serial]
    fn test_validate_config_valid() {
        env::set_var("MODE", "server");
        env::set_var("PORT", "8080");

        let result = validate_config();
        assert!(result.is_ok());

        env::remove_var("MODE");
        env::remove_var("PORT");
    }

    #[test]
    #[serial]
    fn test_validate_config_invalid_mode() {
        env::set_var("MODE", "invalid");

        let result = validate_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid MODE"));

        env::remove_var("MODE");
    }

    #[test]
    #[serial]
    fn test_validate_config_stdio_mode() {
        env::set_var("MODE", "stdio");

        let result = validate_config();
        assert!(result.is_ok());

        env::remove_var("MODE");
    }

    #[test]
    #[serial]
    fn test_validate_config_invalid_port() {
        env::set_var("MODE", "server");
        env::set_var("PORT", "0");

        let result = validate_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PORT cannot be 0"));

        env::remove_var("MODE");
        env::remove_var("PORT");
    }

    #[test]
    #[serial]
    fn validate_config_non_numeric_port_defaults() {
        env::set_var("MODE", "server");
        env::set_var("PORT", "abc");

        let result = validate_config();
        assert!(result.is_ok());

        env::remove_var("MODE");
        env::remove_var("PORT");
    }

    #[tokio::test]
    async fn status_handles_non_200_health_and_tools() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/healthz");
            then.status(500).body("boom");
        });
        server.mock(|when, then| {
            when.method(POST).path("/mcp");
            then.status(500).body("boom");
        });

        let res = show_status(&server.base_url()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_test_grammar_no_url() {
        env::remove_var("GRAMADOIR_BASE_URL");

        let result = test_grammar(None, "test").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No grammar service URL"));
    }

    #[tokio::test]
    async fn test_test_grammar_with_url() {
        // This would need a real grammar service; expect an error
        let result = test_grammar(Some("http://localhost:9999".to_string()), "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_status_handles_unavailable_service() {
        // Should handle errors and return Err when service is down
        let res = show_status("http://localhost:9999").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn run_commands_config_success() {
        let code = run_commands(Commands::Config { validate: true }).await;
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[tokio::test]
    #[serial]
    async fn run_commands_config_failure() {
        env::set_var("MODE", "nope");
        let code = run_commands(Commands::Config { validate: true }).await;
        assert_eq!(code, ExitCode::FAILURE);
        env::remove_var("MODE");
    }

    #[tokio::test]
    #[serial]
    async fn run_commands_health_and_status() {
        // Health should fail against invalid URL and still return FAILURE
        let health = run_commands(Commands::Health { url: "http://localhost:9".into() }).await;
        assert_eq!(health, ExitCode::FAILURE);

        let status = run_commands(Commands::Status { url: "http://localhost:9".into() }).await;
        assert_eq!(status, ExitCode::FAILURE);
    }

    #[tokio::test]
    #[serial]
    async fn run_commands_test_grammar_no_url() {
        env::remove_var("GRAMADOIR_BASE_URL");
        let code = run_commands(Commands::TestGrammar { url: None, text: "abc".into() }).await;
        assert_eq!(code, ExitCode::FAILURE);
    }

    #[tokio::test]
    async fn health_check_ok_and_error_paths() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| { when.method(GET).path("/healthz"); then.status(200); });
        assert!(super::health_check(&server.base_url()).await.is_ok());

        let bad = MockServer::start();
        bad.mock(|when, then| { when.method(GET).path("/healthz"); then.status(500); });
        assert!(super::health_check(&bad.base_url()).await.is_err());
    }

    #[tokio::test]
    async fn show_status_ok_path() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| { when.method(GET).path("/healthz"); then.status(200).body("ok"); });
        server.mock(|when, then| { when.method(POST).path("/mcp"); then.status(200).body("ok"); });
        let res = super::show_status(&server.base_url()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn run_commands_health_success() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| { when.method(GET).path("/healthz"); then.status(200).body("ok"); });
        let code = run_commands(Commands::Health { url: server.base_url() }).await;
        assert_eq!(code, ExitCode::SUCCESS);
    }
}

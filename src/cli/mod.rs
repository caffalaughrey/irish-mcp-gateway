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

    match cli.command {
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
        .timeout(std::time::Duration::from_secs(5))
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
        .timeout(std::time::Duration::from_secs(5))
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

    #[tokio::test]
    async fn test_health_check_success() {
        // This would need a running service, so we'll test the error case
        let result = health_check("http://localhost:9999").await;
        assert!(result.is_err()); // Should fail on non-existent service
    }

    #[test]
    fn test_validate_config_valid() {
        env::set_var("MODE", "server");
        env::set_var("PORT", "8080");

        let result = validate_config();
        assert!(result.is_ok());

        env::remove_var("MODE");
        env::remove_var("PORT");
    }

    #[test]
    fn test_validate_config_invalid_mode() {
        env::set_var("MODE", "invalid");

        let result = validate_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid MODE"));

        env::remove_var("MODE");
    }

    #[test]
    fn test_validate_config_stdio_mode() {
        env::set_var("MODE", "stdio");

        let result = validate_config();
        assert!(result.is_ok());

        env::remove_var("MODE");
    }

    #[test]
    fn test_validate_config_invalid_port() {
        env::set_var("MODE", "server");
        env::set_var("PORT", "0");

        let result = validate_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PORT cannot be 0"));

        env::remove_var("MODE");
        env::remove_var("PORT");
    }

    #[tokio::test]
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
        // This would need a real grammar service, so we'll test the error case
        let result = test_grammar(Some("http://localhost:9999".to_string()), "test").await;
        assert!(result.is_err()); // Should fail on non-existent service
    }
}

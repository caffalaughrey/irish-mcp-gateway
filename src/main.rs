mod api;
mod cli;
mod clients;
mod core;
mod domain;
mod infra;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    infra::logging::init();

    // Check if we're running admin commands
    if std::env::args().len() > 1 {
        let exit_code = cli::run().await;
        std::process::exit(match exit_code {
            std::process::ExitCode::SUCCESS => 0,
            _ => 1, // All other exit codes map to 1
        });
    }
    infra::boot::run_server().await
}

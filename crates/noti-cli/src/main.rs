mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use noti_core::ProviderRegistry;

use output::OutputMode;

/// noti — A unified multi-channel notification CLI for AI agents.
#[derive(Parser, Debug)]
#[command(name = "noti", version, about, long_about = None)]
struct Cli {
    /// Output in JSON format (for machine / agent consumption).
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Send a notification message.
    Send(commands::send::SendArgs),
    /// Manage notification profiles.
    Config(commands::config::ConfigArgs),
    /// List and inspect available providers.
    Providers(commands::providers::ProvidersArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    // Initialize provider registry
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    match &cli.command {
        Commands::Send(args) => commands::send::execute(args, &registry, output).await,
        Commands::Config(args) => commands::config::execute(args, &registry, output).await,
        Commands::Providers(args) => commands::providers::execute(args, &registry, output),
    }
}

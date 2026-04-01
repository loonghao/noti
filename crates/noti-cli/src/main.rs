mod commands;
mod input;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use noti_core::ProviderRegistry;

use output::OutputMode;

/// noti — A unified multi-channel notification CLI for AI agents.
///
/// Designed for both human and agent consumption. Use `--json` or set
/// `NOTI_OUTPUT=json` for structured machine-readable output.
///
/// Agent best practices:
///   - Use `noti schema <provider>` to introspect accepted parameters.
///   - Use `--dry-run` before any mutation to validate inputs.
///   - Use `--fields` to limit response size and protect your context window.
///   - Use `--json` for all programmatic interactions.
#[derive(Parser, Debug)]
#[command(name = "noti", version, about, long_about = None)]
struct Cli {
    /// Output in JSON format (for machine / agent consumption).
    /// Can also be set via NOTI_OUTPUT=json environment variable.
    #[arg(long, global = true)]
    json: bool,

    /// Limit output to specific fields (comma-separated).
    /// Only applies to JSON output. Reduces response size for agent context windows.
    /// Example: --fields provider,success,message
    #[arg(long, global = true, value_delimiter = ',')]
    fields: Option<Vec<String>>,

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
    /// Introspect provider schemas (agent-friendly parameter discovery).
    ///
    /// Use this instead of guessing API parameters from documentation.
    /// The schema output is the canonical source of truth for what
    /// each provider accepts.
    Schema(commands::schema::SchemaArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = OutputMode::detect(cli.json);

    // Initialize provider registry
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let result = match &cli.command {
        Commands::Send(args) => commands::send::execute(args, &registry, output, &cli.fields).await,
        Commands::Config(args) => commands::config::execute(args, &registry, output).await,
        Commands::Providers(args) => commands::providers::execute(args, &registry, output),
        Commands::Schema(args) => commands::schema::execute(args, &registry, output),
    };

    // Unified error handling: ensure JSON errors for agents
    if let Err(ref e) = result {
        if output == OutputMode::Json {
            output::print_error(output, &e.to_string());
            std::process::exit(1);
        }
    }

    result
}

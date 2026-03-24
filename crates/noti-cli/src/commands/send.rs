use anyhow::{Context, Result, bail};
use clap::Args;
use noti_core::{
    AppConfig, Message, MessageFormat, ProviderConfig, ProviderRegistry, parse_notification_url,
};

use crate::output::{OutputMode, print_error, print_json};

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Message text to send.
    #[arg(short, long)]
    pub message: String,

    /// Notification URL (e.g. "wecom://<key>", "slack://<tokens>").
    #[arg(short, long, group = "target")]
    pub to: Option<String>,

    /// Use a saved profile name from config.
    #[arg(short, long, group = "target")]
    pub profile: Option<String>,

    /// Provider name (used with --key and other provider-specific flags).
    #[arg(long, group = "target")]
    pub provider: Option<String>,

    /// Provider-specific key-value parameters (e.g. --param key=value).
    /// Can be specified multiple times.
    #[arg(long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    /// Optional message title / subject.
    #[arg(long)]
    pub title: Option<String>,

    /// Message format: text, markdown, html.
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Request timeout in seconds.
    #[arg(long, default_value = "30")]
    pub timeout: u64,
}

pub async fn execute(
    args: &SendArgs,
    registry: &ProviderRegistry,
    output: OutputMode,
) -> Result<()> {
    // Resolve provider and config
    let (provider_name, config) = resolve_target(args, registry)?;

    // Find the provider
    let provider = registry
        .get_by_name(&provider_name)
        .or_else(|| registry.get_by_scheme(&provider_name))
        .context(format!("unknown provider: {provider_name}"))?;

    // Build message
    let format = args
        .format
        .parse::<MessageFormat>()
        .map_err(|e| anyhow::anyhow!(e))?;

    let mut message = Message::text(&args.message).with_format(format);
    if let Some(ref title) = args.title {
        message = message.with_title(title);
    }

    // Send
    match provider.send(&message, &config).await {
        Ok(response) => {
            match output {
                OutputMode::Json => print_json(&response),
                OutputMode::Human => {
                    if response.success {
                        println!("✓ [{}] {}", response.provider, response.message);
                    } else {
                        eprintln!("✗ [{}] {}", response.provider, response.message);
                    }
                }
            }
            if !response.success {
                std::process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            print_error(output, &e.to_string());
            std::process::exit(1);
        }
    }
}

/// Resolve the target provider name and config from CLI args.
fn resolve_target(
    args: &SendArgs,
    _registry: &ProviderRegistry,
) -> Result<(String, ProviderConfig)> {
    if let Some(ref url) = args.to {
        // URL scheme mode
        let parsed = parse_notification_url(url).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok((parsed.scheme, parsed.config))
    } else if let Some(ref profile_name) = args.profile {
        // Profile mode
        let app_config = AppConfig::load().map_err(|e| anyhow::anyhow!("{e}"))?;
        let profile = app_config
            .get_profile(profile_name)
            .context(format!("profile not found: {profile_name}"))?;
        Ok((profile.provider.clone(), profile.config.clone()))
    } else if let Some(ref provider_name) = args.provider {
        // Direct provider + params mode
        let mut config = ProviderConfig::new();
        for param in &args.params {
            let (k, v) = param.split_once('=').context(format!(
                "invalid param format (expected key=value): {param}"
            ))?;
            config = config.set(k, v);
        }
        Ok((provider_name.clone(), config))
    } else {
        bail!("must specify one of: --to <url>, --profile <name>, or --provider <name>");
    }
}

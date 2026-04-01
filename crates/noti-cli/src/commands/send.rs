use anyhow::{Context, Result, bail};
use clap::Args;
use noti_core::{
    AppConfig, Attachment, Message, MessageFormat, Priority, ProviderConfig, ProviderRegistry,
    parse_notification_url,
};

use crate::input;
use crate::output::{OutputMode, print_error, print_json_filtered};

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Message text to send.
    #[arg(short, long, required_unless_present = "json_payload")]
    pub message: Option<String>,

    /// Send a message from a raw JSON payload (agent-friendly).
    /// Accepts a JSON object with fields: text, title, format, priority, extra.
    /// Example: --json-payload '{"text":"hello","format":"markdown"}'
    #[arg(long, conflicts_with_all = &["message", "title", "format", "priority"])]
    pub json_payload: Option<String>,

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

    /// Message priority: low, normal, high, urgent.
    #[arg(long, default_value = "normal")]
    pub priority: String,

    /// File attachment(s) to send (image, document, etc.).
    /// Can be specified multiple times.
    #[arg(long = "file", short = 'f', value_name = "PATH")]
    pub files: Vec<String>,

    /// Request timeout in seconds.
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Validate inputs and config without actually sending the message.
    /// Use this to verify parameters before executing a real send.
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(
    args: &SendArgs,
    registry: &ProviderRegistry,
    output: OutputMode,
    fields: &Option<Vec<String>>,
) -> Result<()> {
    // --- Input hardening ---
    validate_inputs(args)?;

    // Resolve provider and config
    let (provider_name, config) = resolve_target(args, registry)?;

    // Find the provider
    let provider = registry
        .get_by_name(&provider_name)
        .or_else(|| registry.get_by_scheme(&provider_name))
        .context(format!("unknown provider: {provider_name}"))?;

    // Build message (from --json-payload or individual flags)
    let mut message = build_message(args)?;

    // Attach files
    for file_path in &args.files {
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            bail!("attachment file not found: {file_path}");
        }
        message = message.with_attachment(Attachment::from_path(path));
    }

    // Validate config against provider schema
    if let Err(e) = provider.validate_config(&config) {
        print_error(output, &e.to_string());
        std::process::exit(1);
    }

    // Warn if provider doesn't support attachments but files were given
    if message.has_attachments() && !provider.supports_attachments() {
        match output {
            OutputMode::Json => {
                let warning = serde_json::json!({
                    "status": "warning",
                    "message": format!(
                        "provider '{}' does not support file attachments; files will be ignored",
                        provider.name()
                    )
                });
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&warning).unwrap_or_default()
                );
            }
            OutputMode::Human => {
                eprintln!(
                    "⚠ provider '{}' does not support file attachments; files will be ignored",
                    provider.name()
                );
            }
        }
    }

    // --- Dry run: validate everything, skip actual send ---
    if args.dry_run {
        let dry_run_result = serde_json::json!({
            "status": "dry_run",
            "valid": true,
            "provider": provider.name(),
            "message_preview": {
                "text": message.text,
                "title": message.title,
                "format": message.format.to_string(),
                "priority": message.priority.to_string(),
                "attachment_count": message.attachments.len(),
            },
            "config_keys": config.values.keys().collect::<Vec<_>>(),
        });

        match output {
            OutputMode::Json => print_json_filtered(&dry_run_result, fields),
            OutputMode::Human => {
                println!("✓ dry-run: all inputs valid");
                println!("  provider: {}", provider.name());
                println!("  message: {:?}", message.text);
                if let Some(ref title) = message.title {
                    println!("  title: {title}");
                }
                println!("  format: {}", message.format);
                println!("  priority: {}", message.priority);
                println!("  attachments: {}", message.attachments.len());
            }
        }
        return Ok(());
    }

    // --- Actual send ---
    match provider.send(&message, &config).await {
        Ok(response) => {
            match output {
                OutputMode::Json => print_json_filtered(&response, fields),
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

/// Validate all CLI inputs using input hardening rules.
fn validate_inputs(args: &SendArgs) -> Result<()> {
    if let Some(ref msg) = args.message {
        input::validate_string_input(msg, "--message")?;
    }
    if let Some(ref title) = args.title {
        input::validate_string_input(title, "--title")?;
    }
    if let Some(ref profile) = args.profile {
        input::validate_identifier(profile, "--profile")?;
    }
    if let Some(ref provider) = args.provider {
        input::validate_identifier(provider, "--provider")?;
    }
    for file_path in &args.files {
        input::validate_file_path(file_path, "--file")?;
    }
    for param in &args.params {
        input::validate_string_input(param, "--param")?;
    }
    Ok(())
}

/// Build a Message from either --json-payload or individual CLI flags.
fn build_message(args: &SendArgs) -> Result<Message> {
    if let Some(ref json_str) = args.json_payload {
        // Agent path: parse raw JSON directly into Message fields
        let payload: serde_json::Value =
            serde_json::from_str(json_str).context("invalid JSON in --json-payload")?;

        let text = payload
            .get("text")
            .and_then(|v| v.as_str())
            .context("--json-payload must contain a 'text' field")?;

        let mut message = Message::text(text);

        if let Some(title) = payload.get("title").and_then(|v| v.as_str()) {
            message = message.with_title(title);
        }

        if let Some(format_str) = payload.get("format").and_then(|v| v.as_str()) {
            let format = format_str
                .parse::<MessageFormat>()
                .map_err(|e| anyhow::anyhow!(e))?;
            message = message.with_format(format);
        }

        if let Some(priority_str) = payload.get("priority").and_then(|v| v.as_str()) {
            let priority = priority_str
                .parse::<Priority>()
                .map_err(|e| anyhow::anyhow!(e))?;
            message = message.with_priority(priority);
        }

        // Pass through extra fields
        if let Some(extra) = payload.get("extra").and_then(|v| v.as_object()) {
            for (k, v) in extra {
                message = message.with_extra(k, v.clone());
            }
        }

        Ok(message)
    } else {
        // Human path: build from individual flags
        let text = args
            .message
            .as_deref()
            .context("--message is required when not using --json-payload")?;

        let format = args
            .format
            .parse::<MessageFormat>()
            .map_err(|e| anyhow::anyhow!(e))?;

        let priority = args
            .priority
            .parse::<Priority>()
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut message = Message::text(text)
            .with_format(format)
            .with_priority(priority);

        if let Some(ref title) = args.title {
            message = message.with_title(title);
        }

        Ok(message)
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

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use noti_core::{AppConfig, Profile, ProviderConfig, ProviderRegistry};

use crate::output::{OutputMode, print_error, print_json, print_success};

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Create or update a named profile.
    Set {
        /// Profile name.
        #[arg(long)]
        name: String,
        /// Provider name.
        #[arg(long)]
        provider: String,
        /// Provider-specific key-value parameters (e.g. --param key=value).
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,
        /// Validate the profile without saving it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Display a single profile.
    Get {
        /// Profile name.
        name: String,
    },
    /// List all profiles.
    List,
    /// Remove a profile.
    Remove {
        /// Profile name.
        name: String,
        /// Show what would be removed without actually removing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Test a profile by sending a test message.
    Test {
        /// Profile name.
        name: String,
    },
    /// Show the config file path.
    Path,
}

pub async fn execute(
    args: &ConfigArgs,
    registry: &ProviderRegistry,
    output: OutputMode,
) -> Result<()> {
    match &args.action {
        ConfigAction::Set {
            name,
            provider,
            params,
            dry_run,
        } => {
            let mut config = ProviderConfig::new();
            for param in params {
                let (k, v) = param
                    .split_once('=')
                    .context(format!("invalid param format: {param}"))?;
                config = config.set(k, v);
            }

            // Validate provider exists and config is valid
            if let Some(p) = registry.get_by_name(provider) {
                if let Err(e) = p.validate_config(&config) {
                    print_error(output, &e.to_string());
                    std::process::exit(1);
                }
            }

            if *dry_run {
                let result = serde_json::json!({
                    "status": "dry_run",
                    "valid": true,
                    "profile": name,
                    "provider": provider,
                    "config_keys": config.values.keys().collect::<Vec<_>>(),
                });
                match output {
                    OutputMode::Json => print_json(&result),
                    OutputMode::Human => {
                        println!("✓ dry-run: profile '{name}' is valid (not saved)");
                        println!("  provider: {provider}");
                    }
                }
                return Ok(());
            }

            let profile = Profile {
                provider: provider.clone(),
                config,
            };

            let mut app_config = AppConfig::load().unwrap_or_default();
            app_config.set_profile(name, profile);
            app_config.save().map_err(|e| anyhow::anyhow!("{e}"))?;

            print_success(output, &format!("profile '{name}' saved"));
        }
        ConfigAction::Get { name } => {
            let app_config = AppConfig::load().map_err(|e| anyhow::anyhow!("{e}"))?;
            match app_config.get_profile(name) {
                Some(profile) => match output {
                    OutputMode::Json => print_json(profile),
                    OutputMode::Human => {
                        println!("Profile: {name}");
                        println!("  provider: {}", profile.provider);
                        for (k, v) in &profile.config.values {
                            println!("  {k}: {v}");
                        }
                    }
                },
                None => {
                    print_error(output, &format!("profile not found: {name}"));
                    std::process::exit(2);
                }
            }
        }
        ConfigAction::List => {
            let app_config = AppConfig::load().unwrap_or_default();
            if app_config.profiles.is_empty() {
                match output {
                    OutputMode::Json => print_json(&serde_json::json!([])),
                    OutputMode::Human => println!("No profiles configured."),
                }
            } else {
                match output {
                    OutputMode::Json => {
                        let profiles: serde_json::Value = app_config
                            .profiles
                            .iter()
                            .map(|(name, profile)| {
                                serde_json::json!({
                                    "name": name,
                                    "provider": profile.provider,
                                })
                            })
                            .collect();
                        print_json(&profiles);
                    }
                    OutputMode::Human => {
                        println!("Configured profiles:");
                        let mut names: Vec<&String> = app_config.profiles.keys().collect();
                        names.sort();
                        for name in names {
                            let profile = &app_config.profiles[name];
                            println!("  {name} ({})", profile.provider);
                        }
                    }
                }
            }
        }
        ConfigAction::Remove { name, dry_run } => {
            if *dry_run {
                let app_config = AppConfig::load().unwrap_or_default();
                let exists = app_config.get_profile(name).is_some();
                let result = serde_json::json!({
                    "status": "dry_run",
                    "profile": name,
                    "exists": exists,
                    "would_remove": exists,
                });
                match output {
                    OutputMode::Json => print_json(&result),
                    OutputMode::Human => {
                        if exists {
                            println!("✓ dry-run: profile '{name}' exists and would be removed");
                        } else {
                            println!("✗ dry-run: profile '{name}' not found");
                        }
                    }
                }
                return Ok(());
            }

            let mut app_config = AppConfig::load().unwrap_or_default();
            if app_config.remove_profile(name) {
                app_config.save().map_err(|e| anyhow::anyhow!("{e}"))?;
                print_success(output, &format!("profile '{name}' removed"));
            } else {
                print_error(output, &format!("profile not found: {name}"));
                std::process::exit(2);
            }
        }
        ConfigAction::Test { name } => {
            let app_config = AppConfig::load().map_err(|e| anyhow::anyhow!("{e}"))?;
            let profile = app_config
                .get_profile(name)
                .context(format!("profile not found: {name}"))?;

            let provider = registry
                .get_by_name(&profile.provider)
                .context(format!("unknown provider: {}", profile.provider))?;

            let message = noti_core::Message::text(
                "🔔 noti test message — if you see this, the profile is working!",
            );

            match provider.send(&message, &profile.config).await {
                Ok(response) => {
                    match output {
                        OutputMode::Json => print_json(&response),
                        OutputMode::Human => {
                            if response.success {
                                println!("✓ profile '{name}' is working ({})", response.provider);
                            } else {
                                eprintln!("✗ profile '{name}' test failed: {}", response.message);
                            }
                        }
                    }
                    if !response.success {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    print_error(output, &format!("test failed: {e}"));
                    std::process::exit(1);
                }
            }
        }
        ConfigAction::Path => {
            let path = AppConfig::config_path().map_err(|e| anyhow::anyhow!("{e}"))?;
            match output {
                OutputMode::Json => {
                    print_json(&serde_json::json!({ "path": path.display().to_string() }))
                }
                OutputMode::Human => println!("{}", path.display()),
            }
        }
    }

    Ok(())
}

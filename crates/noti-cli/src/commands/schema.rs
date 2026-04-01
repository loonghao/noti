use anyhow::{Context, Result};
use clap::Args;
use noti_core::ProviderRegistry;

use crate::output::{OutputMode, print_json};

/// Introspect provider schemas at runtime.
///
/// AI agents should use this command instead of guessing API parameters
/// from training data. The output is the canonical source of truth for
/// what each provider accepts.
#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Provider name to introspect (e.g. "slack", "wecom", "telegram").
    pub provider: Option<String>,
}

pub fn execute(
    args: &SchemaArgs,
    registry: &ProviderRegistry,
    output: OutputMode,
) -> Result<()> {
    match &args.provider {
        Some(name) => show_provider_schema(name, registry, output),
        None => show_all_schemas(registry, output),
    }
}

/// Dump the full schema for a single provider.
fn show_provider_schema(
    name: &str,
    registry: &ProviderRegistry,
    output: OutputMode,
) -> Result<()> {
    let provider = registry
        .get_by_name(name)
        .or_else(|| registry.get_by_scheme(name))
        .context(format!("unknown provider: {name}"))?;

    let params: Vec<serde_json::Value> = provider
        .params()
        .iter()
        .map(|p| {
            let mut obj = serde_json::json!({
                "name": p.name,
                "description": p.description,
                "required": p.required,
                "type": "string",
            });
            if let Some(ref ex) = p.example {
                obj["example"] = serde_json::json!(ex);
            }
            obj
        })
        .collect();

    let schema = serde_json::json!({
        "provider": provider.name(),
        "scheme": provider.url_scheme(),
        "description": provider.description(),
        "example_url": provider.example_url(),
        "supports_attachments": provider.supports_attachments(),
        "params": params,
        "send_command": format!(
            "noti send --provider {} {}--message <TEXT>",
            provider.name(),
            provider.params()
                .iter()
                .filter(|p| p.required)
                .map(|p| format!("--param {}=<VALUE> ", p.name))
                .collect::<String>()
        ),
    });

    match output {
        OutputMode::Json => print_json(&schema),
        OutputMode::Human => {
            println!("Schema: {}", provider.name());
            println!("  scheme: {}://", provider.url_scheme());
            println!("  description: {}", provider.description());
            println!("  example_url: {}", provider.example_url());
            println!(
                "  attachments: {}",
                if provider.supports_attachments() {
                    "supported"
                } else {
                    "not supported"
                }
            );
            println!();
            println!("  Parameters:");
            for p in provider.params() {
                let req = if p.required {
                    "REQUIRED"
                } else {
                    "optional"
                };
                print!("    {:<24} [{req}] {}", p.name, p.description);
                if let Some(ref ex) = p.example {
                    print!("  (e.g. \"{ex}\")");
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Dump a summary of all providers and their required params.
fn show_all_schemas(registry: &ProviderRegistry, output: OutputMode) -> Result<()> {
    let providers = registry.all_providers();

    match output {
        OutputMode::Json => {
            let schemas: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| {
                    let params = p.params();
                    let required_params: Vec<&str> = params
                        .iter()
                        .filter(|param| param.required)
                        .map(|param| param.name.as_str())
                        .collect();
                    serde_json::json!({
                        "provider": p.name(),
                        "scheme": p.url_scheme(),
                        "description": p.description(),
                        "required_params": required_params,
                        "supports_attachments": p.supports_attachments(),
                    })
                })
                .collect();
            print_json(&schemas);
        }
        OutputMode::Human => {
            println!("Available provider schemas:");
            println!();
            let mut items: Vec<_> = providers.iter().collect();
            items.sort_by_key(|p| p.name());
            for p in items {
                let params = p.params();
                let required: Vec<&str> = params
                    .iter()
                    .filter(|param| param.required)
                    .map(|param| param.name.as_str())
                    .collect();
                println!(
                    "  {:<14} {}://  required: [{}]",
                    p.name(),
                    p.url_scheme(),
                    required.join(", ")
                );
            }
            println!();
            println!("Use `noti schema <provider>` for full details.");
        }
    }

    Ok(())
}

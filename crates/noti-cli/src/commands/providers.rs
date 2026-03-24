use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use noti_core::ProviderRegistry;

use crate::output::{OutputMode, print_json};

#[derive(Args, Debug)]
pub struct ProvidersArgs {
    #[command(subcommand)]
    pub action: ProvidersAction,
}

#[derive(Subcommand, Debug)]
pub enum ProvidersAction {
    /// List all available notification providers.
    List,
    /// Show detailed information about a specific provider.
    Info {
        /// Provider name.
        name: String,
    },
}

pub fn execute(
    args: &ProvidersArgs,
    registry: &ProviderRegistry,
    output: OutputMode,
) -> Result<()> {
    match &args.action {
        ProvidersAction::List => {
            let providers = registry.all_providers();
            match output {
                OutputMode::Json => {
                    let list: Vec<serde_json::Value> = providers
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "name": p.name(),
                                "scheme": p.url_scheme(),
                                "description": p.description(),
                            })
                        })
                        .collect();
                    print_json(&list);
                }
                OutputMode::Human => {
                    println!("Available providers:");
                    println!();
                    let mut items: Vec<_> = providers
                        .iter()
                        .map(|p| (p.name(), p.url_scheme(), p.description()))
                        .collect();
                    items.sort_by_key(|(name, _, _)| name.to_string());
                    for (name, scheme, desc) in items {
                        println!("  {name:<12} {scheme}://...  {desc}");
                    }
                }
            }
        }
        ProvidersAction::Info { name } => {
            let provider = registry
                .get_by_name(name)
                .context(format!("unknown provider: {name}"))?;

            match output {
                OutputMode::Json => {
                    let params: Vec<serde_json::Value> = provider
                        .params()
                        .iter()
                        .map(|p| {
                            let mut obj = serde_json::json!({
                                "name": p.name,
                                "description": p.description,
                                "required": p.required,
                            });
                            if let Some(ref ex) = p.example {
                                obj["example"] = serde_json::json!(ex);
                            }
                            obj
                        })
                        .collect();
                    print_json(&serde_json::json!({
                        "name": provider.name(),
                        "scheme": provider.url_scheme(),
                        "description": provider.description(),
                        "example_url": provider.example_url(),
                        "params": params,
                    }));
                }
                OutputMode::Human => {
                    println!("Provider: {}", provider.name());
                    println!("  Scheme:      {}://", provider.url_scheme());
                    println!("  Description: {}", provider.description());
                    println!("  Example URL: {}", provider.example_url());
                    println!();
                    println!("  Parameters:");
                    for p in provider.params() {
                        let req = if p.required {
                            "(required)"
                        } else {
                            "(optional)"
                        };
                        print!("    {:<24} {} {}", p.name, req, p.description);
                        if let Some(ref ex) = p.example {
                            print!("  [e.g. {ex}]");
                        }
                        println!();
                    }
                }
            }
        }
    }

    Ok(())
}

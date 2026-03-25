use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Fluxer webhook notification provider.
///
/// Fluxer is a webhook-based notification service that supports rich
/// embeds, text-to-speech, and Discord-style formatting. It can be
/// used in cloud mode (api.fluxer.app) or private self-hosted mode.
///
/// API Reference: <https://fluxer.app>
pub struct FluxerProvider {
    client: Client,
}

impl FluxerProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FluxerProvider {
    fn name(&self) -> &str {
        "fluxer"
    }

    fn url_scheme(&self) -> &str {
        "fluxer"
    }

    fn description(&self) -> &str {
        "Fluxer webhook notifications"
    }

    fn example_url(&self) -> &str {
        "fluxer://<webhook_id>/<webhook_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_id", "Fluxer webhook ID")
                .with_example("417429632418316298"),
            ParamDef::required("webhook_token", "Fluxer webhook token")
                .with_example("JHZ7lQml277CDHmQKMHI8qBe7bk2ZwO5UKjCiOAF7711o33"),
            ParamDef::optional("botname", "Bot display name"),
            ParamDef::optional("avatar_url", "Bot avatar image URL"),
            ParamDef::optional("tts", "Enable text-to-speech (true/false)"),
            ParamDef::optional("host", "Private Fluxer server host (for self-hosted)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_id = config.require("webhook_id", "fluxer")?;
        let webhook_token = config.require("webhook_token", "fluxer")?;

        let base_host = config.get("host").unwrap_or("https://api.fluxer.app");
        let base_url = if base_host.starts_with("http") {
            base_host.to_string()
        } else {
            format!("https://{base_host}")
        };

        let url = format!("{base_url}/webhooks/{webhook_id}/{webhook_token}");

        let mut payload = json!({
            "content": message.text,
        });

        if let Some(ref title) = message.title {
            // Use embed for title + body
            payload = json!({
                "embeds": [{
                    "title": title,
                    "description": message.text,
                }]
            });
        }

        if let Some(botname) = config.get("botname") {
            payload["username"] = json!(botname);
        }

        if let Some(avatar) = config.get("avatar_url") {
            payload["avatar_url"] = json!(avatar);
        }

        if config.get("tts").map(|v| v == "true").unwrap_or(false) {
            payload["tts"] = json!(true);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("fluxer", "webhook notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("fluxer", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

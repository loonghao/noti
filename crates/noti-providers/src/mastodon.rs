use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Mastodon provider.
///
/// Posts a status (toot) via the Mastodon REST API.
///
/// API reference: <https://docs.joinmastodon.org/methods/statuses/#create>
pub struct MastodonProvider {
    client: Client,
}

impl MastodonProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MastodonProvider {
    fn name(&self) -> &str {
        "mastodon"
    }

    fn url_scheme(&self) -> &str {
        "mastodon"
    }

    fn description(&self) -> &str {
        "Mastodon status post (toot) via REST API"
    }

    fn example_url(&self) -> &str {
        "mastodon://<access_token>@<instance>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Mastodon application access token")
                .with_example("your_access_token"),
            ParamDef::required("instance", "Mastodon instance hostname")
                .with_example("mastodon.social"),
            ParamDef::optional(
                "visibility",
                "Post visibility: public, unlisted, private, or direct (default: public)",
            )
            .with_example("unlisted"),
            ParamDef::optional("spoiler_text", "Content warning / spoiler text")
                .with_example("Spoiler!"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "mastodon")?;
        let instance = config.require("instance", "mastodon")?;

        let url = format!("https://{instance}/api/v1/statuses");

        let status_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "status": status_text,
        });

        let visibility = config.get("visibility").unwrap_or("public");
        payload["visibility"] = json!(visibility);

        if let Some(spoiler) = config.get("spoiler_text") {
            payload["spoiler_text"] = json!(spoiler);
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let toot_url = raw.get("url").and_then(|v| v.as_str()).unwrap_or("unknown");
            Ok(
                SendResponse::success("mastodon", format!("toot posted: {toot_url}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("mastodon", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

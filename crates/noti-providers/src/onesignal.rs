use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// OneSignal push notification provider.
///
/// Sends push notifications via the OneSignal REST API.
/// Can target all users, segments, or specific player IDs.
pub struct OneSignalProvider {
    client: Client,
}

impl OneSignalProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for OneSignalProvider {
    fn name(&self) -> &str {
        "onesignal"
    }

    fn url_scheme(&self) -> &str {
        "onesignal"
    }

    fn description(&self) -> &str {
        "OneSignal push notification platform"
    }

    fn example_url(&self) -> &str {
        "onesignal://<app_id>:<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_id", "OneSignal application ID").with_example("your-app-id"),
            ParamDef::required("api_key", "OneSignal REST API key").with_example("your-api-key"),
            ParamDef::optional(
                "include_segments",
                "Target segments (comma-separated, default: Subscribed Users)",
            )
            .with_example("Subscribed Users"),
            ParamDef::optional("player_ids", "Target player IDs (comma-separated)")
                .with_example("player-id-1,player-id-2"),
            ParamDef::optional("url", "URL to open on notification click")
                .with_example("https://example.com"),
            ParamDef::optional("image", "Image URL for the notification")
                .with_example("https://example.com/img.png"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let app_id = config.require("app_id", "onesignal")?;
        let api_key = config.require("api_key", "onesignal")?;

        let url = "https://onesignal.com/api/v1/notifications";

        let mut payload = json!({
            "app_id": app_id,
            "contents": {"en": message.text}
        });

        if let Some(ref title) = message.title {
            payload["headings"] = json!({"en": title});
        }

        // Target: player IDs or segments
        if let Some(player_ids) = config.get("player_ids") {
            let ids: Vec<&str> = player_ids.split(',').map(|s| s.trim()).collect();
            payload["include_player_ids"] = json!(ids);
        } else {
            let segments = config.get("include_segments").unwrap_or("Subscribed Users");
            let seg_list: Vec<&str> = segments.split(',').map(|s| s.trim()).collect();
            payload["included_segments"] = json!(seg_list);
        }

        if let Some(click_url) = config.get("url") {
            payload["url"] = json!(click_url);
        }

        if let Some(image) = config.get("image") {
            payload["big_picture"] = json!(image);
            payload["ios_attachments"] = json!({"image": image});
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Basic {api_key}"))
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
            let recipients = raw.get("recipients").and_then(|v| v.as_u64()).unwrap_or(0);
            Ok(SendResponse::success(
                "onesignal",
                format!("notification sent to {recipients} recipients"),
            )
            .with_status_code(status)
            .with_raw_response(raw))
        } else {
            let errors = raw
                .get("errors")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "unknown error".to_string());
            Ok(
                SendResponse::failure("onesignal", format!("API error: {errors}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

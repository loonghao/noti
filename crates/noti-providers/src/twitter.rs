use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Twitter/X notification provider.
///
/// Uses the X (Twitter) API v2 to post tweets or send DMs.
/// Supports image attachments via the media upload endpoint.
/// API docs: https://developer.x.com/en/docs/x-api
pub struct TwitterProvider {
    client: Client,
}

impl TwitterProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload media via X API v1.1 media/upload (base64) and return the media_id.
    async fn upload_media(
        &self,
        bearer_token: &str,
        data: &[u8],
        mime: &str,
        base_url: Option<&str>,
    ) -> Result<String, NotiError> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);

        let upload_url = if let Some(base) = base_url {
            format!("{base}/1.1/media/upload.json")
        } else {
            "https://upload.twitter.com/1.1/media/upload.json".to_string()
        };

        let resp = self
            .client
            .post(&upload_url)
            .header("Authorization", format!("Bearer {bearer_token}"))
            .form(&[("media_data", b64.as_str()), ("media_category", mime)])
            .send()
            .await
            .map_err(|e| NotiError::Network(format!("media upload failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse upload response: {e}")))?;

        body.get("media_id_string")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                NotiError::provider(
                    "twitter",
                    format!("media upload did not return media_id: {body}"),
                )
            })
    }
}

#[async_trait]
impl NotifyProvider for TwitterProvider {
    fn name(&self) -> &str {
        "twitter"
    }

    fn url_scheme(&self) -> &str {
        "twitter"
    }

    fn description(&self) -> &str {
        "X (Twitter) — post tweets or send DMs via API v2"
    }

    fn example_url(&self) -> &str {
        "twitter://<bearer_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("bearer_token", "X (Twitter) API v2 Bearer token"),
            ParamDef::optional("mode", "Send mode: tweet or dm (default: tweet)"),
            ParamDef::optional("dm_user_id", "Recipient user ID for DM mode"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let bearer_token = config.require("bearer_token", "twitter")?;
        let mode = config.get("mode").unwrap_or("tweet");
        let base_url = config.get("base_url");

        // Upload image attachments and collect media IDs
        let mut media_ids: Vec<String> = Vec::new();
        if message.has_attachments() {
            for attachment in &message.attachments {
                if matches!(
                    attachment.kind,
                    AttachmentKind::Image | AttachmentKind::Video
                ) {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    if let Ok(media_id) = self.upload_media(bearer_token, &data, &mime, base_url).await {
                        media_ids.push(media_id);
                    }
                }
            }
        }

        let (url, payload) = if mode == "dm" {
            let dm_user_id = config.require("dm_user_id", "twitter")?;
            let mut msg_obj = json!({ "text": message.text });
            if !media_ids.is_empty() {
                msg_obj["attachments"] = json!(
                    media_ids
                        .iter()
                        .map(|id| json!({"media_id": id}))
                        .collect::<Vec<_>>()
                );
            }
            let dm_url = if let Some(base) = base_url {
                format!("{base}/2/dm_conversations/with/messages")
            } else {
                "https://api.x.com/2/dm_conversations/with/messages".to_string()
            };
            (
                dm_url,
                json!({
                    "participant_id": dm_user_id,
                    "message": msg_obj
                }),
            )
        } else {
            let mut tweet = json!({ "text": message.text });
            if !media_ids.is_empty() {
                tweet["media"] = json!({ "media_ids": media_ids });
            }
            let tweet_url = if let Some(base) = base_url {
                format!("{base}/2/tweets")
            } else {
                "https://api.x.com/2/tweets".to_string()
            };
            (tweet_url, tweet)
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {bearer_token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("twitter", format!("{mode} sent successfully"))
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("twitter", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}

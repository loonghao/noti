use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// DingTalk (钉钉) group bot webhook provider.
///
/// Supports text, markdown, and actionCard messages.
/// For attachments: images are sent inline via markdown with a public URL if available,
/// or embedded via base64 data URI (best effort — DingTalk may not render these).
/// When `app_key` + `app_secret` are provided, images are uploaded via DingTalk's
/// media upload API to get a proper `@mediaId` reference.
pub struct DingTalkProvider {
    client: Client,
}

impl DingTalkProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(access_token: &str, secret: Option<&str>) -> String {
        let mut url =
            format!("https://oapi.dingtalk.com/robot/send?access_token={access_token}");

        if let Some(secret) = secret {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string();
            let sign = compute_dingtalk_sign(&timestamp, secret);
            url = format!("{url}&timestamp={timestamp}&sign={sign}");
        }
        url
    }

    /// Get DingTalk access token via app credentials.
    async fn get_access_token(
        client: &Client,
        app_key: &str,
        app_secret: &str,
    ) -> Result<String, NotiError> {
        let url = "https://oapi.dingtalk.com/gettoken";
        let resp = client
            .get(url)
            .query(&[("appkey", app_key), ("appsecret", app_secret)])
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("token parse error: {e}")))?;

        let errcode = raw.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode != 0 {
            let errmsg = raw
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(NotiError::provider(
                "dingtalk",
                format!("failed to get access token: {errmsg}"),
            ));
        }

        raw.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                NotiError::provider("dingtalk", "no access_token in response")
            })
    }

    /// Upload media to DingTalk and return the media_id.
    async fn upload_media(
        client: &Client,
        token: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<String, NotiError> {
        let media_type = match attachment.kind {
            AttachmentKind::Image => "image",
            AttachmentKind::Audio => "voice",
            AttachmentKind::Video => "video",
            AttachmentKind::File => "file",
        };

        let url = format!(
            "https://oapi.dingtalk.com/media/upload?access_token={token}&type={media_type}"
        );

        let data = attachment.read_bytes().await?;
        let file_name = attachment.effective_file_name();
        let mime_str = attachment.effective_mime();

        let file_part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name)
            .mime_str(&mime_str)
            .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

        let form = reqwest::multipart::Form::new().part("media", file_part);

        let resp = client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

        let errcode = raw.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode != 0 {
            let errmsg = raw
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(NotiError::provider(
                "dingtalk",
                format!("media upload failed: {errmsg}"),
            ));
        }

        raw.get("media_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                NotiError::provider("dingtalk", "no media_id in upload response")
            })
    }

    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let errcode = raw.get("errcode").and_then(|v| v.as_i64()).unwrap_or(-1);
        if errcode == 0 {
            Ok(
                SendResponse::success("dingtalk", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let errmsg = raw
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("dingtalk", format!("API error: {errmsg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

#[async_trait]
impl NotifyProvider for DingTalkProvider {
    fn name(&self) -> &str {
        "dingtalk"
    }

    fn url_scheme(&self) -> &str {
        "dingtalk"
    }

    fn description(&self) -> &str {
        "DingTalk group bot via webhook"
    }

    fn example_url(&self) -> &str {
        "dingtalk://<access_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "DingTalk bot webhook access token")
                .with_example("abc123def456"),
            ParamDef::optional("secret", "Sign secret for secure mode (SEC...)"),
            ParamDef::optional(
                "app_key",
                "DingTalk App Key (enables media upload via Open API)",
            ),
            ParamDef::optional(
                "app_secret",
                "DingTalk App Secret (required with app_key for uploads)",
            ),
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
        let access_token = config.require("access_token", "dingtalk")?;
        let url = Self::build_url(access_token, config.get("secret"));

        // If there's an image attachment and app credentials, upload via API
        if let Some(img) = message.first_image() {
            if let (Some(app_key), Some(app_secret)) =
                (config.get("app_key"), config.get("app_secret"))
            {
                let token =
                    Self::get_access_token(&self.client, app_key, app_secret).await?;
                let media_id =
                    Self::upload_media(&self.client, &token, img).await?;

                // Send as markdown with uploaded image reference
                let title = message.title.as_deref().unwrap_or("Image");
                let md_text = format!(
                    "### {title}\n\n{}\n\n![image](@mediaId={})",
                    message.text, media_id
                );

                let body = json!({
                    "msgtype": "markdown",
                    "markdown": {
                        "title": title,
                        "text": md_text
                    }
                });

                let resp = self
                    .client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| NotiError::Network(e.to_string()))?;

                return Self::parse_response(resp).await;
            }

            // Fallback: send as markdown with image info (no inline rendering)
            let title = message.title.as_deref().unwrap_or("Image");
            let file_name = img.effective_file_name();
            let md_text = format!(
                "### {title}\n\n{}\n\n🖼️ Image: **{file_name}**",
                message.text
            );

            let body = json!({
                "msgtype": "markdown",
                "markdown": {
                    "title": title,
                    "text": md_text
                }
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        // If there are file attachments, send as actionCard with file info
        if message.has_attachments() {
            let title = message.title.as_deref().unwrap_or("File Notification");
            let mut text = format!("### {title}\n\n{}", message.text);

            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                let kind_label = match attachment.kind {
                    AttachmentKind::Image => "🖼️ Image",
                    AttachmentKind::Audio => "🎵 Audio",
                    AttachmentKind::Video => "🎬 Video",
                    AttachmentKind::File => "📎 File",
                };
                text.push_str(&format!("\n\n{kind_label}: **{file_name}**"));
            }

            let body = json!({
                "msgtype": "actionCard",
                "actionCard": {
                    "title": title,
                    "text": text,
                    "singleTitle": "View Details",
                    "singleURL": ""
                }
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        // Text / Markdown message (no attachments)
        let body = match message.format {
            MessageFormat::Markdown => {
                let title = message.title.as_deref().unwrap_or("Notification");
                json!({
                    "msgtype": "markdown",
                    "markdown": {
                        "title": title,
                        "text": message.text
                    }
                })
            }
            _ => {
                json!({
                    "msgtype": "text",
                    "text": {
                        "content": message.text
                    }
                })
            }
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        Self::parse_response(resp).await
    }
}

/// Compute DingTalk webhook signature using HMAC-SHA256 + Base64.
fn compute_dingtalk_sign(timestamp: &str, secret: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let string_to_sign = format!("{timestamp}\n{secret}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any size");
    mac.update(string_to_sign.as_bytes());
    let result = mac.finalize();
    STANDARD.encode(result.into_bytes())
}

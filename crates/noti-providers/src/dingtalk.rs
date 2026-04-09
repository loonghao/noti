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
        let mut url = format!("https://oapi.dingtalk.com/robot/send?access_token={access_token}");

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
            .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

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
            .ok_or_else(|| NotiError::provider("dingtalk", "no access_token in response"))
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
            .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

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
            .ok_or_else(|| NotiError::provider("dingtalk", "no media_id in upload response"))
    }

    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();

        // Check for 429 rate limiting before parsing body
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::http_helpers::handle_http_error(
                "dingtalk",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

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

    /// Send an actionCard message.
    async fn send_action_card(
        &self,
        message: &Message,
        config: &ProviderConfig,
        url: &str,
    ) -> Result<SendResponse, NotiError> {
        let title = config
            .get("card_title")
            .or(message.title.as_deref())
            .unwrap_or("Notification");
        let text = config.get("card_text").unwrap_or(&message.text);

        let mut action_card = json!({
            "title": title,
            "text": text
        });

        // Single button actionCard
        if let (Some(btn_title), Some(btn_url)) =
            (config.get("card_single_btn"), config.get("card_single_url"))
        {
            action_card["singleTitle"] = json!(btn_title);
            action_card["singleURL"] = json!(btn_url);
        } else if let Some(btns) = config.get("card_btn") {
            // Multi-button actionCard
            let mut btns_array = Vec::new();
            for btn in btns.split(',') {
                let btn = btn.trim();
                if let Some((label, jump_url)) = btn.split_once(':') {
                    btns_array.push(json!({
                        "title": label.trim(),
                        "actionURL": jump_url.trim()
                    }));
                }
            }
            if !btns_array.is_empty() {
                action_card["btns"] = json!(btns_array);
                action_card["btnOrientation"] = json!("1");
            }
        }

        let mut body = json!({
            "msgtype": "actionCard",
            "actionCard": action_card
        });
        body = Self::add_mention_to_body(body, config);

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

        Self::parse_response(resp).await
    }

    /// Send a feedCard message.
    async fn send_feed_card(
        &self,
        message: &Message,
        config: &ProviderConfig,
        url: &str,
    ) -> Result<SendResponse, NotiError> {
        let mut links = Vec::new();

        if let Some(items) = config.get("feed_items") {
            for item in items.split(',') {
                let item = item.trim();
                if let Some((item_title, item_url)) = item.split_once(':') {
                    links.push(json!({
                        "title": item_title.trim(),
                        "messageURL": item_url.trim(),
                        "picURL": ""
                    }));
                }
            }
        }

        // Fallback: use message title and text
        if links.is_empty() {
            let title = message.title.as_deref().unwrap_or("Notification");
            links.push(json!({
                "title": title,
                "messageURL": "",
                "picURL": ""
            }));
        }

        let body = json!({
            "msgtype": "feedCard",
            "feedCard": {
                "links": links
            }
        });

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

        Self::parse_response(resp).await
    }

    /// Add @mention parameters to the request body.
    fn add_mention_to_body(
        mut body: serde_json::Value,
        config: &ProviderConfig,
    ) -> serde_json::Value {
        let mut at = serde_json::Map::new();

        if let Some(mobiles_str) = config.get("mention_mobile") {
            let mobiles: Vec<&str> = mobiles_str.split(',').map(|s| s.trim()).collect();
            at.insert("atMobiles".to_string(), json!(mobiles));
        }

        if let Some(users_str) = config.get("mention_user") {
            let users: Vec<&str> = users_str.split(',').map(|s| s.trim()).collect();
            at.insert("atUserIds".to_string(), json!(users));
        }

        if config.get("mention_all") == Some("true") {
            at.insert("isAtAll".to_string(), json!(true));
        }

        if !at.is_empty() {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("at".to_string(), json!(at));
            }
        }

        body
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
            ParamDef::optional(
                "mention_mobile",
                "Mobile number to @mention (can be repeated)",
            ),
            ParamDef::optional("mention_user", "User ID to @mention (can be repeated)"),
            ParamDef::optional("mention_all", "Mention all users (true/false)"),
            ParamDef::optional("type", "Message type: text, markdown, actionCard, feedCard"),
            ParamDef::optional("card_title", "ActionCard/FeedCard title"),
            ParamDef::optional("card_text", "ActionCard content text"),
            ParamDef::optional(
                "card_btn",
                "ActionCard button (format: label:url, can be repeated)",
            ),
            ParamDef::optional("card_single_btn", "Single button title:url for actionCard"),
            ParamDef::optional("card_single_url", "Single button jump URL"),
            ParamDef::optional(
                "feed_items",
                "FeedCard items (format: title:url, can be repeated)",
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

        // Check for structured actionCard or feedCard
        if let Some(msg_type) = config.get("type") {
            match msg_type {
                "actionCard" => {
                    return self.send_action_card(message, config, &url).await;
                }
                "feedCard" => {
                    return self.send_feed_card(message, config, &url).await;
                }
                _ => {}
            }
        }

        // If there's an image attachment and app credentials, upload via API
        if let Some(img) = message.first_image() {
            if let (Some(app_key), Some(app_secret)) =
                (config.get("app_key"), config.get("app_secret"))
            {
                let token = Self::get_access_token(&self.client, app_key, app_secret).await?;
                let media_id = Self::upload_media(&self.client, &token, img).await?;

                // Send as markdown with uploaded image reference
                let title = message.title.as_deref().unwrap_or("Image");
                let md_text = format!(
                    "### {title}\n\n{}\n\n![image](@mediaId={})",
                    message.text, media_id
                );

                let mut body = json!({
                    "msgtype": "markdown",
                    "markdown": {
                        "title": title,
                        "text": md_text
                    }
                });
                body = Self::add_mention_to_body(body, config);

                let resp = self
                    .client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

                return Self::parse_response(resp).await;
            }

            // Fallback: send as markdown with image info (no inline rendering)
            let title = message.title.as_deref().unwrap_or("Image");
            let file_name = img.effective_file_name();
            let md_text = format!(
                "### {title}\n\n{}\n\n🖼️ Image: **{file_name}**",
                message.text
            );

            let mut body = json!({
                "msgtype": "markdown",
                "markdown": {
                    "title": title,
                    "text": md_text
                }
            });
            body = Self::add_mention_to_body(body, config);

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

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
                .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

            return Self::parse_response(resp).await;
        }

        // Text / Markdown message (no attachments)
        let body = match message.format {
            MessageFormat::Markdown => {
                let title = message.title.as_deref().unwrap_or("Notification");
                let payload = json!({
                    "msgtype": "markdown",
                    "markdown": {
                        "title": title,
                        "text": message.text
                    }
                });
                Self::add_mention_to_body(payload, config)
            }
            _ => {
                let payload = json!({
                    "msgtype": "text",
                    "text": {
                        "content": message.text
                    }
                });
                Self::add_mention_to_body(payload, config)
            }
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("dingtalk", e))?;

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

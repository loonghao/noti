use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Feishu / Lark group bot webhook provider.
///
/// Supports two modes:
/// 1. **Webhook only** (hook_id): Text, markdown, and rich-text post messages.
///    Image attachments are uploaded via the Feishu Open API to obtain an `image_key`,
///    then sent as image messages. File attachments are listed in rich-text posts.
/// 2. **App mode** (app_id + app_secret + hook_id): Enables uploading images and files
///    via the Feishu Open API, then referencing them by key in webhook messages.
pub struct FeishuProvider {
    client: Client,
}

impl FeishuProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_webhook_url(hook_id: &str) -> String {
        format!("https://open.feishu.cn/open-apis/bot/v2/hook/{hook_id}")
    }
}

#[async_trait]
impl NotifyProvider for FeishuProvider {
    fn name(&self) -> &str {
        "feishu"
    }

    fn url_scheme(&self) -> &str {
        "feishu"
    }

    fn description(&self) -> &str {
        "Feishu / Lark group bot via webhook"
    }

    fn example_url(&self) -> &str {
        "feishu://<hook_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("hook_id", "Feishu bot webhook hook ID")
                .with_example("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"),
            ParamDef::optional("secret", "Webhook signature secret for verification"),
            ParamDef::optional(
                "app_id",
                "Feishu App ID (enables image/file upload via Open API)",
            ),
            ParamDef::optional(
                "app_secret",
                "Feishu App Secret (required with app_id for uploads)",
            ),
            ParamDef::optional("mention_user", "User ID to @mention (ou_xxxxx format)"),
            ParamDef::optional("mention_all", "Mention all users (true/false)"),
            ParamDef::optional("type", "Message type: text, post, interactive")
                .with_example("interactive"),
            ParamDef::optional("card_title", "Interactive card title"),
            ParamDef::optional("card_text", "Interactive card content text"),
            ParamDef::optional(
                "card_btn",
                "Card button (format: label:url, can be repeated)",
            ),
            ParamDef::optional("card_json", "Raw card JSON for full control"),
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
        let hook_id = config.require("hook_id", "feishu")?;
        let url = Self::build_webhook_url(hook_id);

        // Check for interactive card with raw JSON
        if let Some(card_json) = config.get("card_json") {
            let parsed: serde_json::Value = serde_json::from_str(card_json)
                .map_err(|e| NotiError::Validation(format!("invalid card JSON: {e}")))?;
            let body = json!({
                "msg_type": "interactive",
                "card": parsed
            });

            let mut request = self.client.post(&url);
            request = Self::maybe_sign(request, &body, config);

            let resp = request
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;
            return Self::parse_response(resp).await;
        }

        // If there's an image attachment, upload via Open API if credentials available
        if let Some(img) = message.first_image() {
            if let (Some(app_id), Some(app_secret)) =
                (config.get("app_id"), config.get("app_secret"))
            {
                // Upload image via Feishu Open API
                let tenant_token =
                    Self::get_tenant_access_token(&self.client, app_id, app_secret).await?;
                let image_key = Self::upload_image(&self.client, &tenant_token, img).await?;

                let body = json!({
                    "msg_type": "image",
                    "content": {
                        "image_key": image_key
                    }
                });

                let mut request = self.client.post(&url);
                request = Self::maybe_sign(request, &body, config);

                let resp = request
                    .send()
                    .await
                    .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

                return Self::parse_response(resp).await;
            }

            // Fallback: send as rich-text post with image info
            let file_name = img.effective_file_name();
            let body = json!({
                "msg_type": "post",
                "content": {
                    "post": {
                        "zh_cn": {
                            "title": message.title.as_deref().unwrap_or("Image"),
                            "content": [
                                [
                                    { "tag": "text", "text": &message.text },
                                ],
                                [
                                    { "tag": "text", "text": format!("🖼️ Image: {file_name}") }
                                ]
                            ]
                        }
                    }
                }
            });

            let mut request = self.client.post(&url);
            request = Self::maybe_sign(request, &body, config);

            let resp = request
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

            return Self::parse_response(resp).await;
        }

        // If there are file attachments, send as rich-text post
        if message.has_attachments() {
            let mut content_lines: Vec<Vec<serde_json::Value>> = Vec::new();
            content_lines.push(vec![json!({ "tag": "text", "text": &message.text })]);

            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                let kind_label = match attachment.kind {
                    AttachmentKind::Image => "🖼️ Image",
                    AttachmentKind::Audio => "🎵 Audio",
                    AttachmentKind::Video => "🎬 Video",
                    AttachmentKind::File => "📎 File",
                };
                content_lines.push(vec![json!({
                    "tag": "text",
                    "text": format!("{kind_label}: {file_name}")
                })]);
            }

            let body = json!({
                "msg_type": "post",
                "content": {
                    "post": {
                        "zh_cn": {
                            "title": message.title.as_deref().unwrap_or("File Notification"),
                            "content": content_lines
                        }
                    }
                }
            });

            let mut request = self.client.post(&url);
            request = Self::maybe_sign(request, &body, config);

            let resp = request
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

            return Self::parse_response(resp).await;
        }

        let body = match message.format {
            MessageFormat::Markdown => {
                // Check for interactive card with structured params
                if config.get("type") == Some("interactive")
                    || config.get("card_title").is_some()
                    || config.get("card_text").is_some()
                    || config.get("card_btn").is_some()
                {
                    Self::build_interactive_card(message, config)
                } else {
                    json!({
                        "msg_type": "interactive",
                        "card": {
                            "header": {
                                "title": {
                                    "tag": "plain_text",
                                    "content": message.title.as_deref().unwrap_or("Notification")
                                }
                            },
                            "elements": [{
                                "tag": "markdown",
                                "content": message.text
                            }]
                        }
                    })
                }
            }
            _ => {
                // Check for @mention first
                if config.get("mention_user").is_some() || config.get("mention_all") == Some("true")
                {
                    Self::build_post_with_mention(message, config)
                } else {
                    json!({
                        "msg_type": "text",
                        "content": {
                            "text": message.text
                        }
                    })
                }
            }
        };

        let mut request = self.client.post(&url);
        request = Self::maybe_sign(request, &body, config);

        let resp = request
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

        Self::parse_response(resp).await
    }
}

impl FeishuProvider {
    fn maybe_sign(
        mut request: reqwest::RequestBuilder,
        body: &serde_json::Value,
        config: &ProviderConfig,
    ) -> reqwest::RequestBuilder {
        if let Some(secret) = config.get("secret") {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string();
            let sign = compute_feishu_sign(&timestamp, secret);
            let mut body_with_sign = body.clone();
            body_with_sign["timestamp"] = json!(timestamp);
            body_with_sign["sign"] = json!(sign);
            request = request.json(&body_with_sign);
        } else {
            request = request.json(body);
        }
        request
    }

    /// Build interactive card payload from structured params.
    fn build_interactive_card(message: &Message, config: &ProviderConfig) -> serde_json::Value {
        let title = config
            .get("card_title")
            .or(message.title.as_deref())
            .unwrap_or("Notification");
        let text = config.get("card_text").unwrap_or(&message.text);
        let msg_type = config.get("type").unwrap_or("interactive");

        if msg_type != "interactive" {
            // Fall back to simple text/post
            return json!({
                "msg_type": msg_type,
                "content": {
                    "text": text
                }
            });
        }

        let mut elements = Vec::new();

        // Add markdown or text content
        elements.push(json!({
            "tag": "markdown",
            "content": text
        }));

        // Add buttons if specified
        if let Some(btns) = config.get("card_btn") {
            let mut btns_array = Vec::new();
            for btn in btns.split(',') {
                let btn = btn.trim();
                if let Some((label, url)) = btn.split_once(':') {
                    btns_array.push(json!({
                        "tag": "action",
                        "actions": [{
                            "tag": "button",
                            "text": {
                                "tag": "plain_text",
                                "content": label.trim()
                            },
                            "type": "primary",
                            "url": url.trim()
                        }]
                    }));
                }
            }
            if !btns_array.is_empty() {
                elements.push(json!({
                    "tag": "div"
                }));
                elements.extend(btns_array);
            }
        }

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": title
                    }
                },
                "elements": elements
            }
        })
    }

    /// Build a post message with optional @mention.
    fn build_post_with_mention(message: &Message, config: &ProviderConfig) -> serde_json::Value {
        let title = message.title.as_deref().unwrap_or("Notification");
        let mut content: Vec<Vec<serde_json::Value>> = Vec::new();

        // Add @mention if specified
        if let Some(user_id) = config.get("mention_user") {
            content.push(vec![json!({
                "tag": "at",
                "user_id": user_id
            })]);
        } else if config.get("mention_all") == Some("true") {
            content.push(vec![json!({
                "tag": "at",
                "user_id": "all"
            })]);
        }

        content.push(vec![json!({ "tag": "text", "text": &message.text })]);

        json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": title,
                        "content": content
                    }
                }
            }
        })
    }

    /// Obtain a tenant_access_token from Feishu Open API.
    async fn get_tenant_access_token(
        client: &Client,
        app_id: &str,
        app_secret: &str,
    ) -> Result<String, NotiError> {
        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
        let body = json!({
            "app_id": app_id,
            "app_secret": app_secret,
        });

        let resp = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("token parse error: {e}")))?;

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(NotiError::provider(
                "feishu",
                format!("failed to get tenant token: {msg}"),
            ));
        }

        raw.get("tenant_access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| NotiError::provider("feishu", "no tenant_access_token in response"))
    }

    /// Upload an image to Feishu Open API and return the image_key.
    async fn upload_image(
        client: &Client,
        tenant_token: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<String, NotiError> {
        let url = "https://open.feishu.cn/open-apis/im/v1/images";
        let data = attachment.read_bytes().await?;
        let file_name = attachment.effective_file_name();
        let mime_str = attachment.effective_mime();

        let file_part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name)
            .mime_str(&mime_str)
            .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .text("image_type", "message")
            .part("image", file_part);

        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {tenant_token}"))
            .multipart(form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("feishu", e))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(NotiError::provider(
                "feishu",
                format!("image upload failed: {msg}"),
            ));
        }

        raw.get("data")
            .and_then(|d| d.get("image_key"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| NotiError::provider("feishu", "no image_key in upload response"))
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
                "feishu",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code == 0 {
            Ok(SendResponse::success("feishu", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("feishu", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

/// Compute Feishu webhook signature using HMAC-SHA256 + Base64.
fn compute_feishu_sign(timestamp: &str, secret: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let string_to_sign = format!("{timestamp}\n{secret}");
    let mut mac =
        Hmac::<Sha256>::new_from_slice(string_to_sign.as_bytes()).expect("HMAC accepts any size");
    mac.update(b"");
    let result = mac.finalize();
    STANDARD.encode(result.into_bytes())
}

use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// WeCom (企业微信) group bot webhook provider.
pub struct WeComProvider {
    client: Client,
}

impl WeComProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Returns the WeCom API base URL, optionally overridden by config.
    fn api_base_url(config: &ProviderConfig) -> String {
        config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://qyapi.weixin.qq.com".to_string())
    }

    fn build_webhook_url(key: &str, config: &ProviderConfig) -> String {
        let base = Self::api_base_url(config);
        format!("{base}/cgi-bin/webhook/send?key={key}")
    }

    fn build_upload_url(key: &str, config: &ProviderConfig) -> String {
        let base = Self::api_base_url(config);
        format!("{base}/cgi-bin/webhook/upload_media?key={key}&type=file")
    }
}

#[async_trait]
impl NotifyProvider for WeComProvider {
    fn name(&self) -> &str {
        "wecom"
    }

    fn url_scheme(&self) -> &str {
        "wecom"
    }

    fn description(&self) -> &str {
        "WeCom (WeChat Work) group bot via webhook"
    }

    fn example_url(&self) -> &str {
        "wecom://<webhook_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("key", "WeCom bot webhook key")
                .with_example("693a91f6-7aoc-4bc4-97a0-0ec2sifa5aaa"),
            ParamDef::optional("mentioned_list", "Comma-separated user IDs to @mention"),
            ParamDef::optional(
                "mentioned_mobile_list",
                "Comma-separated mobile numbers to @mention",
            ),
            ParamDef::optional("type", "Message type: text, markdown, news, template_card")
                .with_example("news"),
            ParamDef::optional("news_title", "News card title"),
            ParamDef::optional("news_desc", "News card description"),
            ParamDef::optional("news_url", "News card jump URL"),
            ParamDef::optional("news_picurl", "News card image URL"),
            ParamDef::optional(
                "card_type",
                "Template card type: text_notice or news_notice",
            )
            .with_example("text_notice"),
            ParamDef::optional("card_title", "Template card main title"),
            ParamDef::optional("card_desc", "Template card description"),
            ParamDef::optional("card_jump_url", "Template card jump URL"),
            ParamDef::optional("card_jump_title", "Template card jump button title"),
            ParamDef::optional(
                "base_url",
                "WeCom API base URL (default: https://qyapi.weixin.qq.com)",
            )
            .with_example("https://qyapi.weixin.qq.com"),
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
        let key = config.require("key", "wecom")?;
        let url = Self::build_webhook_url(key, config);

        // Handle news message type
        if config.get("type") == Some("news") {
            let title = config
                .get("news_title")
                .or(message.title.as_deref())
                .unwrap_or("Notification");
            let mut article = json!({
                "title": title,
                "url": config.get("news_url").unwrap_or(""),
            });
            if let Some(desc) = config.get("news_desc") {
                article["description"] = json!(desc);
            } else if !message.text.is_empty() {
                article["description"] = json!(message.text);
            }
            if let Some(picurl) = config.get("news_picurl") {
                article["picurl"] = json!(picurl);
            }

            let body = json!({
                "msgtype": "news",
                "news": {
                    "articles": [article]
                }
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

            return Self::parse_response(resp).await;
        }

        // Handle template_card message type
        if config.get("type") == Some("template_card") {
            let card_type = config.get("card_type").unwrap_or("text_notice");
            let title = config
                .get("card_title")
                .or(message.title.as_deref())
                .unwrap_or("Notification");
            let desc = config.get("card_desc").unwrap_or(&message.text);

            let mut card = json!({
                "card_type": card_type,
                "main_title": {
                    "title": title,
                    "desc": desc
                }
            });

            if let Some(jump_url) = config.get("card_jump_url") {
                let jump_title = config.get("card_jump_title").unwrap_or("View Details");
                card["card_action"] = json!({
                    "type": 1,
                    "url": jump_url,
                    "title": jump_title
                });
            }

            let body = json!({
                "msgtype": "template_card",
                "template_card": card
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

            return Self::parse_response(resp).await;
        }

        // If has image attachment, try sending as image message
        if let Some(img) = message.first_image() {
            let data = img.read_bytes().await?;
            let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let md5 = md5_hex(&data);

            let body = json!({
                "msgtype": "image",
                "image": {
                    "base64": b64,
                    "md5": md5
                }
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

            return Self::parse_response(resp).await;
        }

        // If has file attachment, upload media first then send file message
        if message.has_attachments() {
            let attachment = &message.attachments[0];
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            let upload_url = Self::build_upload_url(key, config);
            let part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;
            let form = reqwest::multipart::Form::new().part("media", part);

            let upload_resp = self
                .client
                .post(&upload_url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

            let upload_raw: serde_json::Value = upload_resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

            let media_id = upload_raw
                .get("media_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NotiError::provider(
                        "wecom",
                        format!(
                            "media upload failed: {}",
                            upload_raw
                                .get("errmsg")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown error")
                        ),
                    )
                })?;

            let body = json!({
                "msgtype": "file",
                "file": {
                    "media_id": media_id
                }
            });

            let resp = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

            return Self::parse_response(resp).await;
        }

        // Text / Markdown message
        let body = match message.format {
            MessageFormat::Markdown => {
                json!({
                    "msgtype": "markdown",
                    "markdown": {
                        "content": message.text
                    }
                })
            }
            _ => {
                let mut payload = json!({
                    "msgtype": "text",
                    "text": {
                        "content": message.text
                    }
                });
                if let Some(mentioned) = config.get("mentioned_list") {
                    let list: Vec<&str> = mentioned.split(',').map(|s| s.trim()).collect();
                    payload["text"]["mentioned_list"] = json!(list);
                }
                if let Some(mobiles) = config.get("mentioned_mobile_list") {
                    let list: Vec<&str> = mobiles.split(',').map(|s| s.trim()).collect();
                    payload["text"]["mentioned_mobile_list"] = json!(list);
                }
                payload
            }
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("wecom", e))?;

        Self::parse_response(resp).await
    }
}

impl WeComProvider {
    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();

        // Check for 429 rate limiting before parsing body
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = crate::http_helpers::read_response_body("wecom", resp).await;
            return Err(crate::http_helpers::handle_http_error(
                "wecom",
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
            Ok(SendResponse::success("wecom", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let errmsg = raw
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("wecom", format!("API error: {errmsg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

/// Compute MD5 hex digest for WeCom image message.
fn md5_hex(data: &[u8]) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

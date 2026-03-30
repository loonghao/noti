use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Feishu / Lark group bot webhook provider.
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

        // If there's an image attachment, send as image message
        if let Some(img) = message.first_image() {
            let data = img.read_bytes().await?;
            let b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &data,
            );

            let body = json!({
                "msg_type": "image",
                "content": {
                    "image_key": b64
                }
            });

            let mut request = self.client.post(&url);
            request = Self::maybe_sign(request, &body, config);

            let resp = request
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        // If there's a file attachment, send as post message with download link hint
        if message.has_attachments() {
            let attachment = &message.attachments[0];
            let file_name = attachment.effective_file_name();
            let body = json!({
                "msg_type": "post",
                "content": {
                    "post": {
                        "zh_cn": {
                            "title": message.title.as_deref().unwrap_or("File Notification"),
                            "content": [
                                [
                                    {
                                        "tag": "text",
                                        "text": format!("{}\n\n📎 Attachment: {}", message.text, file_name)
                                    }
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
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        let body = match message.format {
            MessageFormat::Markdown => {
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
            _ => {
                json!({
                    "msg_type": "text",
                    "content": {
                        "text": message.text
                    }
                })
            }
        };

        let mut request = self.client.post(&url);
        request = Self::maybe_sign(request, &body, config);

        let resp = request
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

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

    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();
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

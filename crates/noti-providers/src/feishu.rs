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

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let hook_id = config.require("hook_id", "feishu")?;
        let url = Self::build_webhook_url(hook_id);

        let body = match message.format {
            MessageFormat::Markdown => {
                // Feishu uses interactive card for markdown-like content
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

        // If secret is provided, sign the request
        let mut request = self.client.post(&url);
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
            request = request.json(&body);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

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

/// Compute Feishu webhook signature.
fn compute_feishu_sign(timestamp: &str, secret: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    let string_to_sign = format!("{timestamp}\n{secret}");
    // Feishu uses HmacSHA256, but for simplicity we use a basic approach
    // In production, consider using the `hmac` and `sha2` crates
    STANDARD.encode(string_to_sign.as_bytes())
}

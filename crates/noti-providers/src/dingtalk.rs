use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// DingTalk (钉钉) group bot webhook provider.
pub struct DingTalkProvider {
    client: Client,
}

impl DingTalkProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
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
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "dingtalk")?;

        let mut url = format!("https://oapi.dingtalk.com/robot/send?access_token={access_token}");

        // If secret is provided, add signature
        if let Some(secret) = config.get("secret") {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string();
            let sign = compute_dingtalk_sign(&timestamp, secret);
            url = format!("{url}&timestamp={timestamp}&sign={sign}");
        }

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

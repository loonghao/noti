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

    fn build_webhook_url(key: &str) -> String {
        format!("https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={key}")
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
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let key = config.require("key", "wecom")?;
        let url = Self::build_webhook_url(key);

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
                // Add @mentions if provided
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
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
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

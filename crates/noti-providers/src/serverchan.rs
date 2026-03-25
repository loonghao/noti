use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// ServerChan (Server酱) push notification provider.
pub struct ServerChanProvider {
    client: Client,
}

impl ServerChanProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ServerChanProvider {
    fn name(&self) -> &str {
        "serverchan"
    }

    fn url_scheme(&self) -> &str {
        "serverchan"
    }

    fn description(&self) -> &str {
        "ServerChan (Server酱) push to WeChat"
    }

    fn example_url(&self) -> &str {
        "serverchan://<send_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("send_key", "ServerChan SendKey (SCT...)")
                .with_example("SCTxxxxxxxxxxx"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let send_key = config.require("send_key", "serverchan")?;

        let url = format!("https://sctapi.ftqq.com/{send_key}.send");

        let title = message.title.as_deref().unwrap_or("Notification");

        let form = vec![("title", title.to_string()), ("desp", message.text.clone())];

        let resp = self
            .client
            .post(&url)
            .form(&form)
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
            Ok(
                SendResponse::success("serverchan", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("serverchan", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

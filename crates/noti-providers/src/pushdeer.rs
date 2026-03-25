use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// PushDeer cross-platform push notification provider.
pub struct PushDeerProvider {
    client: Client,
}

impl PushDeerProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushDeerProvider {
    fn name(&self) -> &str {
        "pushdeer"
    }

    fn url_scheme(&self) -> &str {
        "pushdeer"
    }

    fn description(&self) -> &str {
        "PushDeer cross-platform push notifications"
    }

    fn example_url(&self) -> &str {
        "pushdeer://<push_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("push_key", "PushDeer push key").with_example("PDU1234TxxxABCD"),
            ParamDef::optional(
                "server",
                "PushDeer server URL (default: https://api2.pushdeer.com)",
            )
            .with_example("https://api2.pushdeer.com"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let push_key = config.require("push_key", "pushdeer")?;
        let server = config.get("server").unwrap_or("https://api2.pushdeer.com");

        let url = format!("{}/message/push", server.trim_end_matches('/'));

        let msg_type = match message.format {
            MessageFormat::Markdown => "markdown",
            MessageFormat::Html => "text",
            MessageFormat::Text => "text",
        };

        let mut form = vec![
            ("pushkey", push_key.to_string()),
            ("text", message.text.clone()),
            ("type", msg_type.to_string()),
        ];

        if let Some(ref title) = message.title {
            form.push(("desp", title.clone()));
        }

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
                SendResponse::success("pushdeer", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pushdeer", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

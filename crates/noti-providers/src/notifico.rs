use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Notifico self-hosted notification service.
///
/// Reference: https://n2.notifico.tech/
pub struct NotificoProvider {
    client: Client,
}

impl NotificoProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NotificoProvider {
    fn name(&self) -> &str {
        "notifico"
    }

    fn url_scheme(&self) -> &str {
        "notifico"
    }

    fn description(&self) -> &str {
        "Notifico self-hosted notification service"
    }

    fn example_url(&self) -> &str {
        "notifico://<project_id>/<msghook>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("project_id", "Notifico project ID"),
            ParamDef::required("msghook", "Notifico message hook token"),
            ParamDef::optional(
                "host",
                "Notifico server URL (default: https://n2.notifico.tech)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let project_id = config.require("project_id", "notifico")?;
        let msghook = config.require("msghook", "notifico")?;
        let host = config.get("host").unwrap_or("https://n2.notifico.tech");

        let url = format!("{host}/hook/{project_id}/{msghook}");

        let body = serde_json::json!({ "payload": message.text });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("notifico", "notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("notifico", format!("Notifico API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

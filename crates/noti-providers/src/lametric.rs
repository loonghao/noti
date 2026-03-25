use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// LaMetric Time smart clock notification provider.
///
/// Sends notifications to LaMetric Time devices via local API or LaMetric Cloud.
pub struct LaMetricProvider {
    client: Client,
}

impl LaMetricProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for LaMetricProvider {
    fn name(&self) -> &str {
        "lametric"
    }

    fn url_scheme(&self) -> &str {
        "lametric"
    }

    fn description(&self) -> &str {
        "LaMetric Time smart clock notifications"
    }

    fn example_url(&self) -> &str {
        "lametric://<api_key>@<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "LaMetric device API key")
                .with_example("your-device-api-key"),
            ParamDef::required("host", "LaMetric device IP or hostname")
                .with_example("192.168.1.100"),
            ParamDef::optional("icon", "Icon ID (e.g., i124, a1234)").with_example("i124"),
            ParamDef::optional("sound", "Notification sound ID").with_example("notification"),
            ParamDef::optional(
                "priority",
                "Priority: info, warning, critical (default: info)",
            )
            .with_example("warning"),
            ParamDef::optional("cycles", "Number of display cycles (default: 1)").with_example("3"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "lametric")?;
        let host = config.require("host", "lametric")?;

        let url = format!("https://{host}:4343/api/v2/device/notifications");

        let icon = config.get("icon").unwrap_or("i124");
        let priority = config.get("priority").unwrap_or("info");
        let cycles = config
            .get("cycles")
            .and_then(|c| c.parse::<u32>().ok())
            .unwrap_or(1);

        let text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let frame = json!({
            "icon": icon,
            "text": text
        });

        let mut notification = json!({
            "priority": priority,
            "icon_type": "none",
            "model": {
                "frames": [frame],
                "cycles": cycles
            }
        });

        if let Some(sound) = config.get("sound") {
            notification["model"]["sound"] = json!({
                "category": "notifications",
                "id": sound
            });
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth("dev", Some(api_key))
            .json(&notification)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("lametric", "notification sent to LaMetric device")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("errors")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("lametric", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

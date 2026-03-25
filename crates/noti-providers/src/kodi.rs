use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Kodi (XBMC) notification provider via JSON-RPC.
///
/// API reference: https://kodi.wiki/view/JSON-RPC_API/v12#GUI.ShowNotification
pub struct KodiProvider {
    client: Client,
}

impl KodiProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for KodiProvider {
    fn name(&self) -> &str {
        "kodi"
    }

    fn url_scheme(&self) -> &str {
        "kodi"
    }

    fn description(&self) -> &str {
        "Kodi (XBMC) GUI notifications via JSON-RPC"
    }

    fn example_url(&self) -> &str {
        "kodi://<user>:<password>@<host>:<port>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Kodi host address (e.g. 192.168.1.100)"),
            ParamDef::optional("port", "Kodi JSON-RPC port (default: 8080)"),
            ParamDef::optional("user", "HTTP basic auth username"),
            ParamDef::optional("password", "HTTP basic auth password"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: http)"),
            ParamDef::optional(
                "display_time",
                "Display time in milliseconds (default: 5000)",
            ),
            ParamDef::optional("image", "Notification icon: info, warning, error, or URL"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let host = config.require("host", "kodi")?;
        let port = config.get("port").unwrap_or("8080");
        let scheme = config.get("scheme").unwrap_or("http");
        let display_time = config
            .get("display_time")
            .unwrap_or("5000")
            .parse::<u32>()
            .unwrap_or(5000);
        let image = config.get("image").unwrap_or("info");
        let title = message.title.as_deref().unwrap_or("noti");

        let url = format!("{scheme}://{host}:{port}/jsonrpc");

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "GUI.ShowNotification",
            "params": {
                "title": title,
                "message": message.text,
                "displaytime": display_time,
                "image": image
            },
            "id": 1
        });

        let mut builder = self.client.post(&url).json(&body);

        if let (Some(user), Some(password)) = (config.get("user"), config.get("password")) {
            builder = builder.basic_auth(user, Some(password));
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("kodi", "notification displayed successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("kodi", format!("Kodi JSON-RPC error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

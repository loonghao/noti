use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Prowl iOS push notification provider.
///
/// Prowl is a push notification client for iOS that receives push
/// notifications from the Prowl API. It supports priority levels,
/// URLs, and application names.
///
/// API docs: <https://www.prowlapp.com/api.php>
pub struct ProwlProvider {
    client: Client,
}

impl ProwlProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ProwlProvider {
    fn name(&self) -> &str {
        "prowl"
    }

    fn url_scheme(&self) -> &str {
        "prowl"
    }

    fn description(&self) -> &str {
        "Prowl iOS push notifications"
    }

    fn example_url(&self) -> &str {
        "prowl://<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Prowl API key")
                .with_example("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
            ParamDef::optional("provider_key", "Provider API key (for higher rate limits)"),
            ParamDef::optional(
                "priority",
                "Priority: -2 (Very Low) to 2 (Emergency), default 0",
            )
            .with_example("1"),
            ParamDef::optional("url", "URL to attach to the notification"),
            ParamDef::optional("application", "Application name (default: noti)")
                .with_example("noti"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "prowl")?;
        let application = config.get("application").unwrap_or("noti");
        let priority = config.get("priority").unwrap_or("0");

        let mut form = vec![
            ("apikey", api_key.to_string()),
            ("application", application.to_string()),
            ("event", message.title.clone().unwrap_or_default()),
            ("description", message.text.clone()),
            ("priority", priority.to_string()),
        ];

        if let Some(provider_key) = config.get("provider_key") {
            form.push(("providerkey", provider_key.to_string()));
        }
        if let Some(url) = config.get("url") {
            form.push(("url", url.to_string()));
        }

        let resp = self
            .client
            .post("https://api.prowlapp.com/publicapi/add")
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("prowl", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(serde_json::json!({"response": body})))
        } else {
            Ok(
                SendResponse::failure("prowl", format!("API error (HTTP {status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        }
    }
}

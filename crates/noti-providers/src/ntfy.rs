use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// ntfy.sh push notification provider.
pub struct NtfyProvider {
    client: Client,
}

impl NtfyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NtfyProvider {
    fn name(&self) -> &str {
        "ntfy"
    }

    fn url_scheme(&self) -> &str {
        "ntfy"
    }

    fn description(&self) -> &str {
        "ntfy.sh push notifications"
    }

    fn example_url(&self) -> &str {
        "ntfy://<topic>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("topic", "ntfy topic name").with_example("my-alerts"),
            ParamDef::optional("server", "ntfy server URL (default: https://ntfy.sh)")
                .with_example("https://ntfy.sh"),
            ParamDef::optional("priority", "Priority: 1-5 (default: 3)").with_example("4"),
            ParamDef::optional("tags", "Comma-separated tags/emojis").with_example("warning,skull"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let topic = config.require("topic", "ntfy")?;
        let server = config.get("server").unwrap_or("https://ntfy.sh");

        let url = format!("{server}/{topic}");

        let mut payload = json!({
            "topic": topic,
            "message": message.text,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if matches!(message.format, MessageFormat::Markdown) {
            payload["markdown"] = json!(true);
        }

        if let Some(priority) = config.get("priority") {
            if let Ok(p) = priority.parse::<u8>() {
                payload["priority"] = json!(p);
            }
        }

        if let Some(tags) = config.get("tags") {
            let tag_list: Vec<&str> = tags.split(',').map(|s| s.trim()).collect();
            payload["tags"] = json!(tag_list);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("ntfy", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("ntfy", format!("API error: {error}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

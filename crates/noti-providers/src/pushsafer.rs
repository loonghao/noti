use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Pushsafer push notification provider.
///
/// Pushsafer sends push notifications to Android, iOS, Windows 10,
/// and various other devices. Supports icons, sounds, vibration, etc.
///
/// API docs: <https://www.pushsafer.com/en/pushapi>
pub struct PushsaferProvider {
    client: Client,
}

impl PushsaferProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushsaferProvider {
    fn name(&self) -> &str {
        "pushsafer"
    }

    fn url_scheme(&self) -> &str {
        "pushsafer"
    }

    fn description(&self) -> &str {
        "Pushsafer push notifications"
    }

    fn example_url(&self) -> &str {
        "pushsafer://<private_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("private_key", "Pushsafer private or alias key"),
            ParamDef::optional("device", "Device or device group ID (default: a = all)")
                .with_example("a"),
            ParamDef::optional("sound", "Sound number (0-62)").with_example("0"),
            ParamDef::optional("vibration", "Vibration pattern (0-3)").with_example("1"),
            ParamDef::optional("icon", "Icon number (1-176)").with_example("1"),
            ParamDef::optional("icon_color", "Icon color hex").with_example("#FF0000"),
            ParamDef::optional("url", "URL to attach"),
            ParamDef::optional("url_title", "Title for the attached URL"),
            ParamDef::optional("priority", "Priority: -2 to 2 (default: 0)").with_example("0"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let private_key = config.require("private_key", "pushsafer")?;
        let device = config.get("device").unwrap_or("a");

        let mut form = vec![
            ("k", private_key.to_string()),
            ("d", device.to_string()),
            ("m", message.text.clone()),
            ("t", message.title.clone().unwrap_or_else(|| "noti".into())),
        ];

        if let Some(sound) = config.get("sound") {
            form.push(("s", sound.to_string()));
        }
        if let Some(vibration) = config.get("vibration") {
            form.push(("v", vibration.to_string()));
        }
        if let Some(icon) = config.get("icon") {
            form.push(("i", icon.to_string()));
        }
        if let Some(icon_color) = config.get("icon_color") {
            form.push(("c", icon_color.to_string()));
        }
        if let Some(url) = config.get("url") {
            form.push(("u", url.to_string()));
        }
        if let Some(url_title) = config.get("url_title") {
            form.push(("ut", url_title.to_string()));
        }
        if let Some(priority) = config.get("priority") {
            form.push(("pr", priority.to_string()));
        }

        // Add image attachments as base64 data URI (up to 3: p, p2, p3)
        if message.has_attachments() {
            let pic_fields = ["p", "p2", "p3"];
            for (i, attachment) in message
                .attachments
                .iter()
                .filter(|a| a.kind == noti_core::AttachmentKind::Image)
                .take(3)
                .enumerate()
            {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                let mime = attachment.effective_mime();
                form.push((pic_fields[i], format!("data:{mime};base64,{b64}")));
            }
        }

        let resp = self
            .client
            .post("https://www.pushsafer.com/api")
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or(serde_json::json!({"error": "failed to parse response"}));

        let api_status = raw.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
        if api_status == 1 {
            Ok(
                SendResponse::success("pushsafer", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pushsafer", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

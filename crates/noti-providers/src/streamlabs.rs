use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Streamlabs alert notification provider.
///
/// API reference: https://dev.streamlabs.com/
pub struct StreamlabsProvider {
    client: Client,
}

impl StreamlabsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for StreamlabsProvider {
    fn name(&self) -> &str {
        "streamlabs"
    }

    fn url_scheme(&self) -> &str {
        "streamlabs"
    }

    fn description(&self) -> &str {
        "Streamlabs stream alerts"
    }

    fn example_url(&self) -> &str {
        "streamlabs://<access_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Streamlabs API access token"),
            ParamDef::optional(
                "type",
                "Alert type: follow, subscription, donation, host (default: follow)",
            ),
            ParamDef::optional("image_href", "Custom image URL for the alert"),
            ParamDef::optional("sound_href", "Custom sound URL for the alert"),
            ParamDef::optional("duration", "Alert display duration in ms (default: 5000)"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let access_token = config.require("access_token", "streamlabs")?;
        let alert_type = config.get("type").unwrap_or("follow");

        let mut form: Vec<(&str, String)> = vec![
            ("access_token", access_token.to_string()),
            ("type", alert_type.to_string()),
            ("message", message.text.clone()),
        ];

        // Handle image from config or attachments
        if let Some(val) = config.get("image_href") {
            form.push(("image_href", val.to_string()));
        } else if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            form.push(("image_href", format!("data:{mime};base64,{b64}")));
        }

        if let Some(val) = config.get("sound_href") {
            form.push(("sound_href", val.to_string()));
        }
        if let Some(val) = config.get("duration") {
            form.push(("duration", val.to_string()));
        }

        let resp = self
            .client
            .post("https://streamlabs.com/api/v1.0/alerts")
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("streamlabs", "alert sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("streamlabs", format!("Streamlabs API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

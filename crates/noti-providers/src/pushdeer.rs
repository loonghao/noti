use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;

/// PushDeer cross-platform push notification provider.
///
/// Supports image attachments via `type=image` with base64 data URI,
/// and markdown-embedded images for other attachment types.
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

    fn supports_attachments(&self) -> bool {
        true
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

        // If there's an image attachment, send as image type
        if let Some(image) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image.read_bytes().await?;
            let mime = image.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let data_uri = format!("data:{mime};base64,{b64}");

            let form = vec![
                ("pushkey", push_key.to_string()),
                ("text", data_uri),
                ("type", "image".to_string()),
            ];

            let resp = self
                .client
                .post(&url)
                .form(&form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        // If there are non-image attachments, embed info in markdown
        let (text, msg_type) = if message.has_attachments() {
            let mut md = message.text.clone();
            for attachment in &message.attachments {
                md.push_str(&format!(
                    "\n\n📎 **Attachment:** {}",
                    attachment.effective_file_name()
                ));
            }
            (md, "markdown")
        } else {
            let mt = match message.format {
                MessageFormat::Markdown => "markdown",
                MessageFormat::Html | MessageFormat::Text => "text",
            };
            (message.text.clone(), mt)
        };

        let mut form = vec![
            ("pushkey", push_key.to_string()),
            ("text", text),
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

        Self::parse_response(resp).await
    }
}

impl PushDeerProvider {
    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
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

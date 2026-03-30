use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// VoIP.ms SMS/MMS provider.
///
/// Uses the VoIP.ms REST API to send SMS and MMS messages.
/// When image attachments are present, switches to the `sendMMS` method
/// with base64-encoded media.
///
/// API docs: https://voip.ms/m/apidocs.php
pub struct VoipMsProvider {
    client: Client,
}

impl VoipMsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for VoipMsProvider {
    fn name(&self) -> &str {
        "voipms"
    }

    fn url_scheme(&self) -> &str {
        "voipms"
    }

    fn description(&self) -> &str {
        "VoIP.ms SMS/MMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "voipms://<email>:<password>@<did>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("email", "VoIP.ms account email").with_example("user@example.com"),
            ParamDef::required("password", "VoIP.ms API password (not account password)"),
            ParamDef::required("did", "Source DID (phone number) for sending SMS")
                .with_example("15551234567"),
            ParamDef::required("to", "Destination phone number").with_example("15559876543"),
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
        let email = config.require("email", "voipms")?;
        let password = config.require("password", "voipms")?;
        let did = config.require("did", "voipms")?;
        let to = config.require("to", "voipms")?;

        let has_image = message.has_attachments()
            && message
                .attachments
                .iter()
                .any(|a| matches!(a.kind, AttachmentKind::Image));

        let (method, msg_type) = if has_image {
            ("sendMMS", "MMS")
        } else {
            ("sendSMS", "SMS")
        };

        let mut url = format!(
            "https://voip.ms/api/v1/rest.php?\
            api_username={email}&api_password={password}&\
            method={method}&did={did}&dst={to}&message={}",
            urlencoding(&message.text)
        );

        // For MMS, append base64-encoded media
        if has_image {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| matches!(a.kind, AttachmentKind::Image))
            {
                let data = img.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let mime = img.effective_mime();
                url.push_str(&format!(
                    "&media1=data:{};base64,{}",
                    urlencoding(&mime),
                    urlencoding(&b64)
                ));
            }
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) && body.contains("\"status\":\"success\"") {
            Ok(
                SendResponse::success("voipms", format!("{msg_type} sent successfully"))
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("voipms", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}

/// Simple percent-encoding for URL query parameter values.
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            ' ' => result.push_str("%20"),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    result
}

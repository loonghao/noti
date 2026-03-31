use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// httpSMS provider.
///
/// Sends SMS/MMS through your Android phone via the httpSMS service.
/// When attachments are present, the message is sent as MMS using the
/// `attachments` field with base64-encoded data URIs.
///
/// API reference: <https://docs.httpsms.com/>
pub struct HttpSmsProvider {
    client: Client,
}

impl HttpSmsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for HttpSmsProvider {
    fn name(&self) -> &str {
        "httpsms"
    }

    fn url_scheme(&self) -> &str {
        "httpsms"
    }

    fn description(&self) -> &str {
        "httpSMS — send SMS/MMS via Android phone"
    }

    fn example_url(&self) -> &str {
        "httpsms://<api_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "httpSMS API key"),
            ParamDef::required("from", "Sender phone number (your Android phone)")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional("encrypt", "Enable end-to-end encryption: true or false"),
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
        let api_key = config.require("api_key", "httpsms")?;
        let from = config.require("from", "httpsms")?;
        let to = config.require("to", "httpsms")?;

        let url = "https://api.httpsms.com/v1/messages/send";

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "content": text,
            "from": from,
            "to": to,
        });

        if let Some(encrypt) = config.get("encrypt") {
            if encrypt == "true" {
                payload["encrypted"] = json!(true);
            }
        }

        // Add attachments as base64 data URIs to trigger MMS delivery
        if message.has_attachments() {
            let mut attachment_uris = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let mime = attachment.effective_mime();
                attachment_uris.push(format!("data:{mime};base64,{b64}"));
            }
            payload["attachments"] = json!(attachment_uris);
        }

        let resp = self
            .client
            .post(url)
            .header("x-api-key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let api_status = raw.get("status").and_then(|v| v.as_str()).unwrap_or("");

        if api_status == "pending"
            || api_status == "sent"
            || (200..300).contains(&(status as usize))
        {
            Ok(
                SendResponse::success("httpsms", "message queued via httpSMS")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("httpsms", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

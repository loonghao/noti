use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Matrix messaging provider via the Client-Server API.
///
/// Posts messages to a Matrix room using an access token.
/// The homeserver URL, room ID, and access token are required.
///
/// Supports plain text, Markdown (as `org.matrix.custom.html`), and HTML.
/// Supports file/image uploads via the media upload endpoint.
pub struct MatrixProvider {
    client: Client,
}

impl MatrixProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn base_url(config: &ProviderConfig) -> String {
        let server = config.get("server").unwrap_or("matrix.org");
        let port = config.get("port").unwrap_or("443");
        let url_scheme = config.get("scheme").unwrap_or("https");
        format!("{url_scheme}://{server}:{port}")
    }

    fn txn_id() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    }
}

#[async_trait]
impl NotifyProvider for MatrixProvider {
    fn name(&self) -> &str {
        "matrix"
    }

    fn url_scheme(&self) -> &str {
        "matrix"
    }

    fn description(&self) -> &str {
        "Matrix via Client-Server API"
    }

    fn example_url(&self) -> &str {
        "matrix://<access_token>/<room_id>?server=matrix.org"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Matrix access token").with_example("syt_xxx_yyy"),
            ParamDef::required("room_id", "Target room ID (e.g. !abc:matrix.org)")
                .with_example("!abc123:matrix.org"),
            ParamDef::optional("server", "Homeserver host (default: matrix.org)")
                .with_example("matrix.org"),
            ParamDef::optional("port", "Homeserver port (default: 443)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
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
        let access_token = config.require("access_token", "matrix")?;
        let room_id = config.require("room_id", "matrix")?;
        let base = Self::base_url(config);

        let encoded_room_id = room_id
            .replace('!', "%21")
            .replace(':', "%3A")
            .replace('#', "%23");

        // If attachments, upload media first then send message referencing it
        if message.has_attachments() {
            // Send text message first if there's text
            if !message.text.is_empty() {
                let txn_id = Self::txn_id();
                let text_url = format!(
                    "{base}/_matrix/client/v3/rooms/{encoded_room_id}/send/m.room.message/{txn_id}"
                );

                let text_payload = match message.format {
                    MessageFormat::Html | MessageFormat::Markdown => {
                        json!({
                            "msgtype": "m.text",
                            "body": message.text,
                            "format": "org.matrix.custom.html",
                            "formatted_body": message.text
                        })
                    }
                    MessageFormat::Text => {
                        json!({
                            "msgtype": "m.text",
                            "body": message.text
                        })
                    }
                };

                let _ = self
                    .client
                    .put(&text_url)
                    .header("Authorization", format!("Bearer {access_token}"))
                    .json(&text_payload)
                    .send()
                    .await;
            }

            // Upload and send each attachment
            let mut last_response = None;
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let mime_str = attachment.effective_mime();

                // Upload media
                let upload_url = format!(
                    "{base}/_matrix/media/v3/upload?filename={}",
                    urlencoding_simple(&file_name)
                );

                let upload_resp = self
                    .client
                    .post(&upload_url)
                    .header("Authorization", format!("Bearer {access_token}"))
                    .header("Content-Type", &mime_str)
                    .body(data)
                    .send()
                    .await
                    .map_err(|e| crate::http_helpers::classify_reqwest_error("matrix", e))?;

                let upload_raw: serde_json::Value = upload_resp
                    .json()
                    .await
                    .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

                let content_uri = upload_raw
                    .get("content_uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        NotiError::provider("matrix", "no content_uri in upload response")
                    })?;

                let msgtype = match attachment.kind {
                    AttachmentKind::Image => "m.image",
                    AttachmentKind::Audio => "m.audio",
                    AttachmentKind::Video => "m.video",
                    AttachmentKind::File => "m.file",
                };

                let txn_id = Self::txn_id();
                let send_url = format!(
                    "{base}/_matrix/client/v3/rooms/{encoded_room_id}/send/m.room.message/{txn_id}"
                );

                let payload = json!({
                    "msgtype": msgtype,
                    "body": file_name,
                    "url": content_uri,
                    "info": {
                        "mimetype": mime_str
                    }
                });

                let resp = self
                    .client
                    .put(&send_url)
                    .header("Authorization", format!("Bearer {access_token}"))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| crate::http_helpers::classify_reqwest_error("matrix", e))?;

                let status = resp.status().as_u16();
                let raw: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

                if !(200..300).contains(&status) {
                    let error_msg = raw
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    return Ok(
                        SendResponse::failure("matrix", format!("API error: {error_msg}"))
                            .with_status_code(status)
                            .with_raw_response(raw),
                    );
                }
                last_response = Some((status, raw));
            }

            if let Some((status, raw)) = last_response {
                return Ok(SendResponse::success("matrix", "file(s) sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw));
            }
        }

        // Text message
        let txn_id = Self::txn_id();
        let url = format!(
            "{base}/_matrix/client/v3/rooms/{encoded_room_id}/send/m.room.message/{txn_id}"
        );

        let payload = match message.format {
            MessageFormat::Html | MessageFormat::Markdown => {
                json!({
                    "msgtype": "m.text",
                    "body": message.text,
                    "format": "org.matrix.custom.html",
                    "formatted_body": message.text
                })
            }
            MessageFormat::Text => {
                json!({
                    "msgtype": "m.text",
                    "body": message.text
                })
            }
        };

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("matrix", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("matrix", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("matrix", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

/// Simple percent-encoding for file names in URL query.
fn urlencoding_simple(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                format!("{}", b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

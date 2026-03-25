use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Matrix messaging provider via the Client-Server API.
///
/// Posts messages to a Matrix room using an access token.
/// The homeserver URL, room ID, and access token are required.
///
/// Supports plain text, Markdown (as `org.matrix.custom.html`), and HTML.
pub struct MatrixProvider {
    client: Client,
}

impl MatrixProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
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

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "matrix")?;
        let room_id = config.require("room_id", "matrix")?;
        let server = config.get("server").unwrap_or("matrix.org");
        let port = config.get("port").unwrap_or("443");
        let url_scheme = config.get("scheme").unwrap_or("https");

        // URL-encode the room ID (it contains ! and :)
        let encoded_room_id = room_id
            .replace('!', "%21")
            .replace(':', "%3A")
            .replace('#', "%23");

        // Generate a transaction ID
        let txn_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let url = format!(
            "{url_scheme}://{server}:{port}/_matrix/client/v3/rooms/{encoded_room_id}/send/m.room.message/{txn_id}"
        );

        // Build the message body based on format
        let payload = match message.format {
            MessageFormat::Html => {
                json!({
                    "msgtype": "m.text",
                    "body": message.text,
                    "format": "org.matrix.custom.html",
                    "formatted_body": message.text
                })
            }
            MessageFormat::Markdown => {
                // Send markdown as HTML formatted body
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
            .map_err(|e| NotiError::Network(e.to_string()))?;

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

use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Enigma2 (OpenWebif) notification provider.
///
/// Enigma2 is a Linux-based firmware for satellite receivers (Dreambox, VU+, etc).
/// Notifications are sent to the TV screen via the OpenWebif REST API.
///
/// API Reference: <https://github.com/E2OpenPlugins/e2openplugin-OpenWebif>
pub struct Enigma2Provider {
    client: Client,
}

impl Enigma2Provider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for Enigma2Provider {
    fn name(&self) -> &str {
        "enigma2"
    }

    fn url_scheme(&self) -> &str {
        "enigma2"
    }

    fn description(&self) -> &str {
        "Enigma2 satellite receiver on-screen notifications via OpenWebif"
    }

    fn example_url(&self) -> &str {
        "enigma2://<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Enigma2 device hostname/IP").with_example("192.168.1.50"),
            ParamDef::optional("port", "HTTP port (default: 80)").with_example("80"),
            ParamDef::optional("user", "HTTP auth username"),
            ParamDef::optional("password", "HTTP auth password"),
            ParamDef::optional("scheme", "http or https (default: http)").with_example("http"),
            ParamDef::optional(
                "timeout",
                "Display duration in seconds (default: 13, -1 for permanent)",
            )
            .with_example("13"),
            ParamDef::optional(
                "msg_type",
                "Message type: 1=yes/no, 2=info, 3=message, 4=attention (default: 1)",
            )
            .with_example("1"),
            ParamDef::optional("base_url", "Override base URL for the Enigma2 device")
                .with_example("http://192.168.1.50:80"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "enigma2")?;
        let port = config.get("port").unwrap_or("80");
        let scheme = config.get("scheme").unwrap_or("http");
        let timeout = config.get("timeout").unwrap_or("13");
        let msg_type = config.get("msg_type").unwrap_or("1");

        let text = if let Some(ref title) = message.title {
            format!("{}\n{}", title, message.text)
        } else {
            message.text.clone()
        };

        // URL-encode the text
        let encoded_text = urlencoded(&text);

        let default_base = format!("{scheme}://{host}:{port}");
        let base_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or(default_base);

        let url = format!(
            "{base_url}/api/message?text={encoded_text}&type={msg_type}&timeout={timeout}"
        );

        let mut request = self.client.get(&url);

        // Add basic auth if credentials provided
        if let Some(user) = config.get("user") {
            let password = config.get("password").unwrap_or("");
            request = request.basic_auth(user, Some(password));
        }

        let resp = request
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("enigma2", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        let result = raw.get("result").and_then(|v| v.as_bool()).unwrap_or(false);

        if result || (200..300).contains(&status) {
            Ok(
                SendResponse::success("enigma2", "on-screen notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("enigma2", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

/// Simple percent-encoding for URL query parameters.
fn urlencoded(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for ch in s.bytes() {
        match ch {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(ch as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{ch:02X}"));
            }
        }
    }
    result
}

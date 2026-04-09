use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// DAPNET (Decentralized Amateur Paging Network) provider.
///
/// DAPNET is a network for ham radio operators that supports paging via the
/// POCSAG protocol. Messages can be sent via the DAPNET REST API.
///
/// API Reference: <https://hampager.de/dokuwiki/>
pub struct DapnetProvider {
    client: Client,
}

impl DapnetProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for DapnetProvider {
    fn name(&self) -> &str {
        "dapnet"
    }

    fn url_scheme(&self) -> &str {
        "dapnet"
    }

    fn description(&self) -> &str {
        "DAPNET ham radio paging network"
    }

    fn example_url(&self) -> &str {
        "dapnet://<callsign>:<password>@<to_callsign>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("callsign", "Your DAPNET login callsign").with_example("DL1ABC"),
            ParamDef::required("password", "Your DAPNET API password"),
            ParamDef::required("to", "Recipient callsign").with_example("DL2DEF"),
            ParamDef::optional("txgroup", "Transmitter group (default: dl-all)")
                .with_example("dl-all"),
            ParamDef::optional(
                "emergency",
                "Emergency priority (true/false, default: false)",
            ),
            ParamDef::optional("base_url", "Override base URL for the DAPNET API")
                .with_example("https://hampager.de"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let callsign = config.require("callsign", "dapnet")?;
        let password = config.require("password", "dapnet")?;
        let to = config.require("to", "dapnet")?;

        let txgroup = config.get("txgroup").unwrap_or("dl-all");
        let emergency = config
            .get("emergency")
            .is_some_and(|v| v == "true" || v == "1");

        let default_url = "https://hampager.de/api/calls";
        let url = config
            .get("base_url")
            .map(|s| {
                let base = s.trim_end_matches('/');
                format!("{base}/api/calls")
            })
            .unwrap_or_else(|| default_url.to_string());

        // Truncate message to POCSAG max (80 chars)
        let text = if message.text.len() > 80 {
            &message.text[..80]
        } else {
            &message.text
        };

        let payload = json!({
            "text": text,
            "callSignNames": [to],
            "transmitterGroupNames": [txgroup],
            "emergency": emergency,
        });

        // DAPNET uses HTTP Basic Auth
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("{callsign}:{password}"));

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Basic {credentials}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("dapnet", "paging message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("dapnet", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// MQTT notification provider (via HTTP publish API).
///
/// Publishes messages to an MQTT broker that supports HTTP REST API
/// (e.g., EMQX, HiveMQ, Mosquitto with HTTP plugin).
/// This avoids needing a native MQTT client library.
pub struct MqttProvider {
    client: Client,
}

impl MqttProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MqttProvider {
    fn name(&self) -> &str {
        "mqtt"
    }

    fn url_scheme(&self) -> &str {
        "mqtt"
    }

    fn description(&self) -> &str {
        "MQTT publish via broker HTTP API (EMQX, HiveMQ, etc.)"
    }

    fn example_url(&self) -> &str {
        "mqtt://<user>:<password>@<host>/<topic>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "host",
                "MQTT broker HTTP API host (e.g. broker.emqx.io:18083)",
            )
            .with_example("broker.emqx.io:18083"),
            ParamDef::required("topic", "MQTT topic to publish to").with_example("noti/alerts"),
            ParamDef::optional("user", "Username for MQTT broker authentication"),
            ParamDef::optional("password", "Password for MQTT broker authentication"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: http)")
                .with_example("http"),
            ParamDef::optional("qos", "Quality of Service level: 0, 1, or 2 (default: 0)")
                .with_example("0"),
            ParamDef::optional("retain", "Retain message: true or false (default: false)"),
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
        let host = config.require("host", "mqtt")?;
        let topic = config.require("topic", "mqtt")?;

        let scheme = config.get("scheme").unwrap_or("http");
        let qos = config.get("qos").unwrap_or("0").parse::<u8>().unwrap_or(0);
        let retain = config.get("retain").map(|r| r == "true").unwrap_or(false);

        // EMQX-style HTTP publish API
        let url = format!("{scheme}://{host}/api/v5/publish");

        // Build payload: if attachments present, use structured JSON with base64 data
        let mqtt_payload = if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                attachments_json.push(json!({
                    "filename": attachment.effective_file_name(),
                    "mime_type": attachment.effective_mime(),
                    "data": b64,
                }));
            }
            serde_json::to_string(&json!({
                "message": message.text,
                "title": message.title,
                "attachments": attachments_json,
            }))
            .unwrap_or_else(|_| message.text.clone())
        } else {
            message.text.clone()
        };

        let payload = json!({
            "topic": topic,
            "payload": mqtt_payload,
            "qos": qos,
            "retain": retain,
        });

        let mut req = self.client.post(&url).json(&payload);

        if let (Some(user), Some(password)) = (config.get("user"), config.get("password")) {
            req = req.basic_auth(user, Some(password));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("mqtt", "message published successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(SendResponse::failure("mqtt", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({ "body": body })))
        }
    }
}

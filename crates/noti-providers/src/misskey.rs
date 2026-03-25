use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Misskey fediverse note posting provider.
///
/// Posts notes to a Misskey instance via its API.
/// Misskey is an open-source decentralized microblogging platform.
pub struct MisskeyProvider {
    client: Client,
}

impl MisskeyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MisskeyProvider {
    fn name(&self) -> &str {
        "misskey"
    }

    fn url_scheme(&self) -> &str {
        "misskey"
    }

    fn description(&self) -> &str {
        "Misskey fediverse note posting"
    }

    fn example_url(&self) -> &str {
        "misskey://<access_token>@<instance>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Misskey API access token")
                .with_example("your-access-token"),
            ParamDef::required("instance", "Misskey instance hostname").with_example("misskey.io"),
            ParamDef::optional(
                "visibility",
                "Note visibility: public, home, followers, specified (default: public)",
            )
            .with_example("public"),
            ParamDef::optional("cw", "Content warning / subject text").with_example("spoiler"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "misskey")?;
        let instance = config.require("instance", "misskey")?;

        let url = format!("https://{instance}/api/notes/create");

        let visibility = config.get("visibility").unwrap_or("public");

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "i": access_token,
            "text": text,
            "visibility": visibility
        });

        if let Some(cw) = config.get("cw") {
            payload["cw"] = json!(cw);
        }

        // Use title as CW if set and no explicit CW
        if config.get("cw").is_none() {
            if let Some(ref title) = message.title {
                // Don't set cw, title is already in text
                let _ = title;
            }
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let note_id = raw
                .get("createdNote")
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Ok(
                SendResponse::success("misskey", format!("note posted (id: {note_id})"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("misskey", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

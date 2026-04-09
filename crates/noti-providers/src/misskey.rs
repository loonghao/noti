use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Misskey fediverse note posting provider.
///
/// Posts notes to a Misskey instance via its API.
/// Misskey is an open-source decentralized microblogging platform.
/// Supports file attachments via the Drive API.
pub struct MisskeyProvider {
    client: Client,
}

impl MisskeyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file to Misskey Drive and return the file ID.
    async fn upload_to_drive(
        &self,
        base_url: &str,
        access_token: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<String, NotiError> {
        let url = format!("{base_url}/api/drive/files/create");
        let data = attachment.read_bytes().await?;
        let file_name = attachment.effective_file_name();
        let mime_str = attachment.effective_mime();

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name)
            .mime_str(&mime_str)
            .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .text("i", access_token.to_string())
            .part("file", part);

        let resp = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("misskey", e))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

        raw.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                let err = raw
                    .get("error")
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                NotiError::provider("misskey", format!("drive upload failed: {err}"))
            })
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
            ParamDef::optional("base_url", "Misskey API base URL override (default: https://{instance})")
                .with_example("https://misskey.io"),
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
        let access_token = config.require("access_token", "misskey")?;
        let instance = config.require("instance", "misskey")?;

        let base_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("https://{instance}"));

        let url = format!("{base_url}/api/notes/create");

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

        // Upload attachments to Drive and attach file IDs
        if message.has_attachments() {
            let mut file_ids = Vec::new();
            for attachment in &message.attachments {
                let file_id = self
                    .upload_to_drive(&base_url, access_token, attachment)
                    .await?;
                file_ids.push(file_id);
            }
            payload["fileIds"] = json!(file_ids);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("misskey", e))?;

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

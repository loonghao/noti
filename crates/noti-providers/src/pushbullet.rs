use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// PushBullet push notification provider.
///
/// Sends push notifications via the PushBullet API.
/// Supports pushing to all devices or a specific device/channel.
/// Supports file uploads via the upload-request flow.
pub struct PushBulletProvider {
    client: Client,
}

impl PushBulletProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushBulletProvider {
    fn name(&self) -> &str {
        "pushbullet"
    }

    fn url_scheme(&self) -> &str {
        "pushbullet"
    }

    fn description(&self) -> &str {
        "PushBullet cross-platform push notifications"
    }

    fn example_url(&self) -> &str {
        "pushbullet://<access_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "PushBullet access token")
                .with_example("o.abcdef1234567890"),
            ParamDef::optional("device_iden", "Target device identifier"),
            ParamDef::optional("channel_tag", "Channel tag to push to"),
            ParamDef::optional("email", "Email of recipient to push to"),
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
        let access_token = config.require("access_token", "pushbullet")?;

        let url = "https://api.pushbullet.com/v2/pushes";
        let title = message.title.as_deref().unwrap_or("Notification");

        // If there's a file attachment, use the upload flow
        if message.has_attachments() {
            let attachment = &message.attachments[0];
            let file_name = attachment.effective_file_name();
            let file_type = attachment.effective_mime();

            // Step 1: Request upload URL
            let upload_req = json!({
                "file_name": file_name,
                "file_type": file_type,
            });

            let upload_resp = self
                .client
                .post("https://api.pushbullet.com/v2/upload-request")
                .header("Access-Token", access_token)
                .json(&upload_req)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("pushbullet", e))?;

            let upload_data: serde_json::Value = upload_resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("upload request parse error: {e}")))?;

            let upload_url = upload_data
                .get("upload_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| NotiError::provider("pushbullet", "no upload_url in response"))?;

            let file_url = upload_data
                .get("file_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| NotiError::provider("pushbullet", "no file_url in response"))?;

            // Step 2: Upload the file
            let data = attachment.read_bytes().await?;
            let file_part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name.clone())
                .mime_str(&file_type)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let form = reqwest::multipart::Form::new().part("file", file_part);

            self.client
                .post(upload_url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("pushbullet", e))?;

            // Step 3: Send file push
            let mut payload = json!({
                "type": "file",
                "title": title,
                "body": message.text,
                "file_name": file_name,
                "file_type": file_type,
                "file_url": file_url,
            });

            if let Some(device) = config.get("device_iden") {
                payload["device_iden"] = json!(device);
            }
            if let Some(channel) = config.get("channel_tag") {
                payload["channel_tag"] = json!(channel);
            }
            if let Some(email) = config.get("email") {
                payload["email"] = json!(email);
            }

            let resp = self
                .client
                .post(url)
                .header("Access-Token", access_token)
                .json(&payload)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("pushbullet", e))?;

            let status = resp.status().as_u16();
            let raw: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

            return if (200..300).contains(&status) && raw.get("iden").is_some() {
                Ok(
                    SendResponse::success("pushbullet", "file push sent successfully")
                        .with_status_code(status)
                        .with_raw_response(raw),
                )
            } else {
                let error = raw
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error");
                Ok(
                    SendResponse::failure("pushbullet", format!("API error: {error}"))
                        .with_status_code(status)
                        .with_raw_response(raw),
                )
            };
        }

        // Text-only note push
        let mut payload = json!({
            "type": "note",
            "title": title,
            "body": message.text,
        });

        if let Some(device) = config.get("device_iden") {
            payload["device_iden"] = json!(device);
        }
        if let Some(channel) = config.get("channel_tag") {
            payload["channel_tag"] = json!(channel);
        }
        if let Some(email) = config.get("email") {
            payload["email"] = json!(email);
        }

        let resp = self
            .client
            .post(url)
            .header("Access-Token", access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("pushbullet", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) && raw.get("iden").is_some() {
            Ok(
                SendResponse::success("pushbullet", "push sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pushbullet", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

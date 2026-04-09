use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Nextcloud Talk chat message provider.
///
/// Supports file attachments via upload to Nextcloud WebDAV and sharing in Talk.
///
/// API reference: https://nextcloud-talk.readthedocs.io/en/latest/chat/
pub struct NcTalkProvider {
    client: Client,
}

impl NcTalkProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file to Nextcloud via WebDAV and share it in Talk.
    async fn upload_and_share(
        &self,
        base_url: &str,
        user: &str,
        password: &str,
        room_token: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<(), NotiError> {
        let file_name = attachment.effective_file_name();
        let data = attachment.read_bytes().await?;

        // Upload to Nextcloud WebDAV (Talk attachments folder)
        let upload_path = format!("Talk/{file_name}");
        let dav_url = format!("{base_url}/remote.php/dav/files/{user}/{upload_path}");

        let upload_resp = self
            .client
            .put(&dav_url)
            .basic_auth(user, Some(password))
            .header("Content-Type", &attachment.effective_mime())
            .body(data)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let upload_status = upload_resp.status().as_u16();
        if !(200..300).contains(&upload_status) {
            return Err(NotiError::provider(
                "nctalk",
                format!("file upload failed (HTTP {upload_status})"),
            ));
        }

        // Share file in Talk room via the share API
        let share_url = format!("{base_url}/ocs/v2.php/apps/files_sharing/api/v1/shares");

        let share_body = serde_json::json!({
            "shareType": 10,
            "shareWith": room_token,
            "path": format!("/{upload_path}"),
        });

        let share_resp = self
            .client
            .post(&share_url)
            .basic_auth(user, Some(password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&share_body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let share_status = share_resp.status().as_u16();
        if !(200..300).contains(&share_status) {
            return Err(NotiError::provider(
                "nctalk",
                format!("file share failed (HTTP {share_status})"),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl NotifyProvider for NcTalkProvider {
    fn name(&self) -> &str {
        "nctalk"
    }

    fn url_scheme(&self) -> &str {
        "nctalk"
    }

    fn description(&self) -> &str {
        "Nextcloud Talk chat messaging via OCS API"
    }

    fn example_url(&self) -> &str {
        "nctalk://<user>:<password>@<host>/<room_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user", "Nextcloud username"),
            ParamDef::required("password", "Nextcloud password or app token"),
            ParamDef::required("host", "Nextcloud server host (e.g. cloud.example.com)"),
            ParamDef::required("room_token", "Talk room/conversation token"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
            ParamDef::optional("base_url", "Nextcloud base URL override (default: {scheme}://{host})")
                .with_example("https://cloud.example.com"),
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

        let user = config.require("user", "nctalk")?;
        let password = config.require("password", "nctalk")?;
        let host = config.require("host", "nctalk")?;
        let room_token = config.require("room_token", "nctalk")?;
        let scheme = config.get("scheme").unwrap_or("https");

        let base_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("{scheme}://{host}"));

        // Send text message first
        let url = format!("{base_url}/ocs/v2.php/apps/spreed/api/v1/chat/{room_token}");

        let body = serde_json::json!({ "message": message.text });

        let resp = self
            .client
            .post(&url)
            .basic_auth(user, Some(password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if !(200..300).contains(&status) {
            return Ok(
                SendResponse::failure("nctalk", format!("Nextcloud Talk API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            );
        }

        // Upload and share file attachments
        if message.has_attachments() {
            for attachment in &message.attachments {
                self.upload_and_share(&base_url, user, password, room_token, attachment)
                    .await?;
            }
        }

        Ok(SendResponse::success("nctalk", "message sent successfully")
            .with_status_code(status)
            .with_raw_response(raw))
    }
}

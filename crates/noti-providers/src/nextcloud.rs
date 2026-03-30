use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Nextcloud push notification provider.
///
/// Sends notifications via the Nextcloud admin notifications API.
/// Supports file attachments via WebDAV upload.
/// Requires an admin account with the notifications app enabled.
///
/// API reference: <https://github.com/nextcloud/notifications/blob/master/docs/admin-notifications.md>
pub struct NextcloudProvider {
    client: Client,
}

impl NextcloudProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NextcloudProvider {
    fn name(&self) -> &str {
        "nextcloud"
    }

    fn url_scheme(&self) -> &str {
        "ncloud"
    }

    fn description(&self) -> &str {
        "Nextcloud push notifications via OCS API"
    }

    fn example_url(&self) -> &str {
        "ncloud://<user>:<password>@<host>/<target_user>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user", "Nextcloud admin username").with_example("admin"),
            ParamDef::required("password", "Nextcloud password or app token")
                .with_example("your-app-token"),
            ParamDef::required("host", "Nextcloud server hostname")
                .with_example("cloud.example.com"),
            ParamDef::optional("target_user", "User to notify (defaults to self)")
                .with_example("john"),
            ParamDef::optional("scheme", "HTTP scheme: https or http").with_example("https"),
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
        let user = config.require("user", "nextcloud")?;
        let password = config.require("password", "nextcloud")?;
        let host = config.require("host", "nextcloud")?;

        let target_user = config.get("target_user").unwrap_or(user);
        let scheme = config.get("scheme").unwrap_or("https");

        // Upload attachments via WebDAV if present
        let mut uploaded_files: Vec<String> = Vec::new();
        for attachment in &message.attachments {
            if let Ok(data) = attachment.read_bytes().await {
                let file_name = attachment.effective_file_name();
                let webdav_url = format!(
                    "{scheme}://{host}/remote.php/dav/files/{user}/noti-attachments/{file_name}"
                );

                // Ensure directory exists (MKCOL)
                let _ = self
                    .client
                    .request(
                        reqwest::Method::from_bytes(b"MKCOL").unwrap_or(reqwest::Method::PUT),
                        format!("{scheme}://{host}/remote.php/dav/files/{user}/noti-attachments/"),
                    )
                    .basic_auth(user, Some(password))
                    .send()
                    .await;

                // Upload file via WebDAV PUT
                let upload_resp = self
                    .client
                    .put(&webdav_url)
                    .basic_auth(user, Some(password))
                    .header("Content-Type", attachment.effective_mime())
                    .body(data)
                    .send()
                    .await;

                if let Ok(resp) = upload_resp {
                    if resp.status().is_success() || resp.status().as_u16() == 201 {
                        uploaded_files.push(file_name);
                    }
                }
            }
        }

        let url = format!(
            "{scheme}://{host}/ocs/v2.php/apps/admin_notifications/api/v1/notifications/userToNotify/{target_user}"
        );

        let short_message = message.title.as_deref().unwrap_or("Notification");

        // Append uploaded file references to the message
        let mut long_message = message.text.clone();
        if !uploaded_files.is_empty() {
            long_message.push_str("\n\nAttachments:");
            for fname in &uploaded_files {
                long_message.push_str(&format!("\n- /noti-attachments/{fname}"));
            }
        }

        let payload = json!({
            "shortMessage": short_message,
            "longMessage": long_message,
        });

        let resp = self
            .client
            .post(&url)
            .basic_auth(user, Some(password))
            .header("OCS-APIREQUEST", "true")
            .header("Accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("nextcloud", "notification sent via Nextcloud")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("ocs")
                .and_then(|v| v.get("meta"))
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("nextcloud", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

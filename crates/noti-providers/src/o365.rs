use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Office 365 / Outlook email via Microsoft Graph API.
///
/// API reference: https://learn.microsoft.com/en-us/graph/api/user-sendmail
pub struct O365Provider {
    client: Client,
}

impl O365Provider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for O365Provider {
    fn name(&self) -> &str {
        "o365"
    }

    fn url_scheme(&self) -> &str {
        "o365"
    }

    fn description(&self) -> &str {
        "Office 365 / Outlook email via Microsoft Graph API"
    }

    fn example_url(&self) -> &str {
        "o365://<client_id>:<client_secret>@<tenant_id>/<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("tenant_id", "Azure AD tenant ID"),
            ParamDef::required("client_id", "Azure AD application (client) ID"),
            ParamDef::required("client_secret", "Azure AD client secret"),
            ParamDef::required(
                "from",
                "Sender email address (must be a valid user in tenant)",
            ),
            ParamDef::required("to", "Recipient email address"),
            ParamDef::optional("cc", "CC email address"),
            ParamDef::optional("bcc", "BCC email address"),
            ParamDef::optional(
                "save_to_sent",
                "Save to Sent Items (true/false, default: true)",
            ),
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

        let tenant_id = config.require("tenant_id", "o365")?;
        let client_id = config.require("client_id", "o365")?;
        let client_secret = config.require("client_secret", "o365")?;
        let from = config.require("from", "o365")?;
        let to = config.require("to", "o365")?;
        let subject = message.title.as_deref().unwrap_or("Notification from noti");
        let save_to_sent = config.get("save_to_sent").unwrap_or("true") == "true";

        // Step 1: Get access token via client credentials
        let token_url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");

        let token_resp = self
            .client
            .post(&token_url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("scope", "https://graph.microsoft.com/.default"),
            ])
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let token_data: serde_json::Value =
            token_resp.json().await.unwrap_or(serde_json::Value::Null);

        let access_token = token_data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NotiError::provider("o365", format!("failed to get access token: {token_data}"))
            })?;

        // Step 2: Send email via Graph API
        let send_url = format!("https://graph.microsoft.com/v1.0/users/{from}/sendMail");

        let to_recipients = vec![serde_json::json!({
            "emailAddress": { "address": to }
        })];

        let mut cc_recipients = vec![];
        if let Some(cc) = config.get("cc") {
            cc_recipients.push(serde_json::json!({
                "emailAddress": { "address": cc }
            }));
        }

        let mut bcc_recipients = vec![];
        if let Some(bcc) = config.get("bcc") {
            bcc_recipients.push(serde_json::json!({
                "emailAddress": { "address": bcc }
            }));
        }

        // Build attachments for Graph API
        let mut graph_attachments = Vec::new();
        if message.has_attachments() {
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                graph_attachments.push(serde_json::json!({
                    "@odata.type": "#microsoft.graph.fileAttachment",
                    "name": attachment.effective_file_name(),
                    "contentType": attachment.effective_mime(),
                    "contentBytes": b64,
                }));
            }
        }

        let body = serde_json::json!({
            "message": {
                "subject": subject,
                "body": { "contentType": "Text", "content": message.text },
                "toRecipients": to_recipients,
                "ccRecipients": cc_recipients,
                "bccRecipients": bcc_recipients,
                "attachments": graph_attachments
            },
            "saveToSentItems": save_to_sent
        });

        let resp = self
            .client
            .post(&send_url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(SendResponse::success("o365", "email sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("o365", format!("Microsoft Graph API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Jira notification provider — adds comments and attachments to issues.
///
/// Supports file attachments via the Jira attachment REST API.
///
/// API reference:
/// - Comments: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-comments/
/// - Attachments: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-attachments/
pub struct JiraProvider {
    client: Client,
}

impl JiraProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for JiraProvider {
    fn name(&self) -> &str {
        "jira"
    }

    fn url_scheme(&self) -> &str {
        "jira"
    }

    fn description(&self) -> &str {
        "Jira issue comment via REST API v3"
    }

    fn example_url(&self) -> &str {
        "jira://<user>:<api_token>@<host>/<issue_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Jira instance URL (e.g. mycompany.atlassian.net)"),
            ParamDef::required("user", "Jira user email"),
            ParamDef::required("api_token", "Jira API token"),
            ParamDef::required("issue_key", "Issue key to comment on (e.g. PROJ-123)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
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

        let host = config.require("host", "jira")?;
        let user = config.require("user", "jira")?;
        let api_token = config.require("api_token", "jira")?;
        let issue_key = config.require("issue_key", "jira")?;
        let scheme = config.get("scheme").unwrap_or("https");

        // Step 1: Upload attachments if present
        if message.has_attachments() {
            let attach_url = format!("{scheme}://{host}/rest/api/3/issue/{issue_key}/attachments");

            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let mime_str = attachment.effective_mime();

                let part = reqwest::multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str(&mime_str)
                    .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

                let form = reqwest::multipart::Form::new().part("file", part);

                let resp = self
                    .client
                    .post(&attach_url)
                    .basic_auth(user, Some(api_token))
                    .header("X-Atlassian-Token", "no-check")
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| NotiError::Network(e.to_string()))?;

                let status = resp.status().as_u16();
                if !(200..300).contains(&status) {
                    let raw: serde_json::Value =
                        resp.json().await.unwrap_or(serde_json::Value::Null);
                    return Ok(SendResponse::failure(
                        "jira",
                        format!("attachment upload failed (HTTP {status})"),
                    )
                    .with_status_code(status)
                    .with_raw_response(raw));
                }
            }
        }

        // Step 2: Add comment
        let comment_url = format!("{scheme}://{host}/rest/api/3/issue/{issue_key}/comment");

        let body = serde_json::json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": message.text
                    }]
                }]
            }
        });

        let resp = self
            .client
            .post(&comment_url)
            .basic_auth(user, Some(api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            let msg = if message.has_attachments() {
                "comment and attachments added successfully"
            } else {
                "comment added successfully"
            };
            Ok(SendResponse::success("jira", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("jira", format!("Jira API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

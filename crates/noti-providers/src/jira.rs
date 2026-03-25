use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Jira notification provider — adds comments to issues.
///
/// API reference: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-comments/
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

        let url = format!("{scheme}://{host}/rest/api/3/issue/{issue_key}/comment");

        // Atlassian Document Format (ADF) for API v3
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
            .post(&url)
            .basic_auth(user, Some(api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(SendResponse::success("jira", "comment added successfully")
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

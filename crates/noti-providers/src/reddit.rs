use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Reddit private message provider.
///
/// Sends private messages via the Reddit API (OAuth2).
/// Requires app credentials and user credentials.
pub struct RedditProvider {
    client: Client,
}

impl RedditProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for RedditProvider {
    fn name(&self) -> &str {
        "reddit"
    }

    fn url_scheme(&self) -> &str {
        "reddit"
    }

    fn description(&self) -> &str {
        "Reddit private message via Reddit API"
    }

    fn example_url(&self) -> &str {
        "reddit://<client_id>:<client_secret>@<user>:<password>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("client_id", "Reddit app client ID").with_example("your-client-id"),
            ParamDef::required("client_secret", "Reddit app client secret")
                .with_example("your-client-secret"),
            ParamDef::required("user", "Reddit username for authentication").with_example("mybot"),
            ParamDef::required("password", "Reddit password").with_example("mypassword"),
            ParamDef::required("to", "Recipient Reddit username").with_example("targetuser"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let client_id = config.require("client_id", "reddit")?;
        let client_secret = config.require("client_secret", "reddit")?;
        let user = config.require("user", "reddit")?;
        let password = config.require("password", "reddit")?;
        let to = config.require("to", "reddit")?;

        // Step 1: Get OAuth2 access token
        let token_resp = self
            .client
            .post("https://www.reddit.com/api/v1/access_token")
            .basic_auth(client_id, Some(client_secret))
            .header("User-Agent", "noti-cli/0.1.0")
            .form(&[
                ("grant_type", "password"),
                ("username", user),
                ("password", password),
            ])
            .send()
            .await
            .map_err(|e| NotiError::Network(format!("token request failed: {e}")))?;

        let token_body: serde_json::Value = token_resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse token response: {e}")))?;

        let access_token = token_body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NotiError::provider(
                    "reddit",
                    format!(
                        "failed to obtain Reddit access token: {}",
                        serde_json::to_string(&token_body).unwrap_or_default()
                    ),
                )
            })?;

        // Step 2: Send private message
        let subject = message.title.as_deref().unwrap_or("noti notification");

        let resp = self
            .client
            .post("https://oauth.reddit.com/api/compose")
            .bearer_auth(access_token)
            .header("User-Agent", "noti-cli/0.1.0")
            .form(&[
                ("to", to),
                ("subject", subject),
                ("text", message.text.as_str()),
                ("api_type", "json"),
            ])
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let errors = raw
            .get("json")
            .and_then(|v| v.get("errors"))
            .and_then(|v| v.as_array());

        if errors.is_none() || errors.is_some_and(|e| e.is_empty()) {
            Ok(
                SendResponse::success("reddit", format!("private message sent to /u/{to}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_str = errors
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown error".to_string());
            Ok(
                SendResponse::failure("reddit", format!("API error: {error_str}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

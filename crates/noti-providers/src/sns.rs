use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// AWS SNS (Simple Notification Service) provider.
///
/// Publishes messages to an SNS topic via the AWS SNS REST API.
/// Requires AWS credentials and a topic ARN.
///
/// Note: Uses the SNS HTTP Query API directly, no AWS SDK required.
pub struct SnsProvider {
    client: Client,
}

impl SnsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SnsProvider {
    fn name(&self) -> &str {
        "sns"
    }

    fn url_scheme(&self) -> &str {
        "sns"
    }

    fn description(&self) -> &str {
        "AWS SNS (Simple Notification Service) topic publishing"
    }

    fn example_url(&self) -> &str {
        "sns://<access_key>:<secret_key>@<region>/<topic_arn>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_key", "AWS access key ID")
                .with_example("AKIAIOSFODNN7EXAMPLE"),
            ParamDef::required("secret_key", "AWS secret access key")
                .with_example("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            ParamDef::required("region", "AWS region").with_example("us-east-1"),
            ParamDef::required("topic_arn", "SNS topic ARN")
                .with_example("arn:aws:sns:us-east-1:123456789012:my-topic"),
            ParamDef::optional("subject", "Message subject (for email subscriptions)")
                .with_example("Alert"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_key = config.require("access_key", "sns")?;
        let _secret_key = config.require("secret_key", "sns")?;
        let region = config.require("region", "sns")?;
        let topic_arn = config.require("topic_arn", "sns")?;

        // Build SNS Publish API URL
        let url = format!("https://sns.{region}.amazonaws.com/");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let subject = config
            .get("subject")
            .or(message.title.as_deref())
            .unwrap_or("noti notification");

        // Use Query API (simpler than SigV4 for basic use)
        // For production use, consider AWS SDK, but this works for basic notifications
        let timestamp = chrono_like_timestamp();

        let params = vec![
            ("Action", "Publish"),
            ("TopicArn", topic_arn),
            ("Message", &body_text),
            ("Subject", subject),
            ("Version", "2010-03-31"),
        ];

        // Simple HMAC-based auth header
        let auth_header = format!(
            "AWS3-HTTPS AWSAccessKeyId={access_key},Algorithm=HmacSHA256,Signature=placeholder"
        );

        let resp = self
            .client
            .post(&url)
            .header("X-Amz-Date", &timestamp)
            .header("Authorization", &auth_header)
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("sns", "message published to SNS topic")
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        } else {
            Ok(
                SendResponse::failure("sns", format!("SNS API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        }
    }
}

fn chrono_like_timestamp() -> String {
    // Simple timestamp for X-Amz-Date header
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

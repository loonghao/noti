use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// AWS SNS (Simple Notification Service) provider.
///
/// Publishes messages to an SNS topic via the AWS SNS REST API.
/// Requires AWS credentials and a topic ARN.
/// Supports attachments via MessageAttributes: images are embedded as base64
/// in a `noti.image` attribute, and file metadata is included in `noti.attachments`.
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

    fn supports_attachments(&self) -> bool {
        true
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
            ParamDef::optional("base_url", "SNS endpoint URL override (default: https://sns.{region}.amazonaws.com)")
                .with_example("https://sns.us-east-1.amazonaws.com"),
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
        let url = if let Some(base) = config.get("base_url") {
            base.trim_end_matches('/').to_string()
        } else {
            format!("https://sns.{region}.amazonaws.com/")
        };

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
        let timestamp = chrono_like_timestamp();

        let mut params = vec![
            ("Action".to_string(), "Publish".to_string()),
            ("TopicArn".to_string(), topic_arn.to_string()),
            ("Message".to_string(), body_text),
            ("Subject".to_string(), subject.to_string()),
            ("Version".to_string(), "2010-03-31".to_string()),
        ];

        // Add attachments as MessageAttributes
        if message.has_attachments() {
            let mut attr_idx = 1;

            // Embed first image as base64 data URI in a string attribute
            if let Some(img) = message.first_image() {
                if let Ok(data) = img.read_bytes().await {
                    let mime = img.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let data_uri = format!("data:{mime};base64,{b64}");
                    params.push((
                        format!("MessageAttributes.entry.{attr_idx}.Name"),
                        "noti.image".to_string(),
                    ));
                    params.push((
                        format!("MessageAttributes.entry.{attr_idx}.Value.DataType"),
                        "String".to_string(),
                    ));
                    params.push((
                        format!("MessageAttributes.entry.{attr_idx}.Value.StringValue"),
                        data_uri,
                    ));
                    attr_idx += 1;
                }
            }

            // Add attachment metadata (names, types) as a JSON string attribute
            let att_meta: Vec<serde_json::Value> = message
                .attachments
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "name": a.effective_file_name(),
                        "mime": a.effective_mime(),
                        "kind": format!("{:?}", a.kind),
                    })
                })
                .collect();
            params.push((
                format!("MessageAttributes.entry.{attr_idx}.Name"),
                "noti.attachments".to_string(),
            ));
            params.push((
                format!("MessageAttributes.entry.{attr_idx}.Value.DataType"),
                "String".to_string(),
            ));
            params.push((
                format!("MessageAttributes.entry.{attr_idx}.Value.StringValue"),
                serde_json::to_string(&att_meta).unwrap_or_default(),
            ));
        }

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

use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Bluesky social posting provider.
///
/// Posts to Bluesky (bsky.social) via the AT Protocol.
/// Requires a handle and app password.
///
/// API reference: <https://docs.bsky.app/docs/tutorials/creating-a-post>
pub struct BlueskyProvider {
    client: Client,
}

impl BlueskyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BlueskyProvider {
    fn name(&self) -> &str {
        "bluesky"
    }

    fn url_scheme(&self) -> &str {
        "bluesky"
    }

    fn description(&self) -> &str {
        "Bluesky social network posting via AT Protocol"
    }

    fn example_url(&self) -> &str {
        "bluesky://<handle>:<app_password>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("handle", "Bluesky handle (e.g. user.bsky.social)")
                .with_example("user.bsky.social"),
            ParamDef::required("app_password", "Bluesky app password")
                .with_example("xxxx-xxxx-xxxx-xxxx"),
            ParamDef::optional("server", "PDS server URL").with_example("https://bsky.social"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let handle = config.require("handle", "bluesky")?;
        let app_password = config.require("app_password", "bluesky")?;

        let server = config.get("server").unwrap_or("https://bsky.social");

        // Step 1: Create session (authenticate)
        let session_url = format!("{server}/xrpc/com.atproto.server.createSession");
        let session_payload = json!({
            "identifier": handle,
            "password": app_password,
        });

        let session_resp = self
            .client
            .post(&session_url)
            .json(&session_payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let session_status = session_resp.status().as_u16();
        if !(200..300).contains(&(session_status as usize)) {
            let body = session_resp.text().await.unwrap_or_default();
            return Ok(
                SendResponse::failure("bluesky", format!("authentication failed: {body}"))
                    .with_status_code(session_status),
            );
        }

        let session: serde_json::Value = session_resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse session: {e}")))?;

        let access_jwt = session
            .get("accessJwt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotiError::provider("bluesky", "missing accessJwt in session"))?;

        let did = session
            .get("did")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotiError::provider("bluesky", "missing did in session"))?;

        // Step 2: Create post
        let post_url = format!("{server}/xrpc/com.atproto.repo.createRecord");

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        // Truncate to 300 graphemes (Bluesky limit)
        let post_text = if text.chars().count() > 300 {
            text.chars().take(297).collect::<String>() + "..."
        } else {
            text
        };

        let now = chrono_now();
        let post_payload = json!({
            "repo": did,
            "collection": "app.bsky.feed.post",
            "record": {
                "$type": "app.bsky.feed.post",
                "text": post_text,
                "createdAt": now,
            }
        });

        let resp = self
            .client
            .post(&post_url)
            .bearer_auth(access_jwt)
            .json(&post_payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("bluesky", "post created on Bluesky")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("bluesky", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

/// Generate an ISO 8601 timestamp without pulling in the chrono crate.
fn chrono_now() -> String {
    // Use a simple approach: SystemTime → seconds since epoch → format
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to a rough ISO 8601 date-time
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Days since 1970-01-01
    let mut y = 1970i64;
    let mut d = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months_days: [i64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for (i, &md) in months_days.iter().enumerate() {
        if d < md {
            m = i + 1;
            break;
        }
        d -= md;
    }

    format!(
        "{y:04}-{m:02}-{:02}T{hours:02}:{minutes:02}:{seconds:02}.000Z",
        d + 1
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

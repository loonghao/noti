use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use p256::ecdsa::{signature::Signer, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use reqwest::Client;
use serde_json::json;

/// Web Push (VAPID) provider.
///
/// Sends browser push notifications using the Web Push protocol.
/// Supports VAPID authentication for identifying the sender.
///
/// Note: This implementation supports VAPID authentication but sends payload
/// as plain text (not encrypted). For full Web Push support with encrypted
/// payloads, consider using a service like Firebase Cloud Messaging (FCM).
///
/// Reference: <https://web.dev/push-notifications-overview/>
pub struct WebPushProvider {
    client: Client,
}

impl WebPushProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VAPID Authentication
// ─────────────────────────────────────────────────────────────────────────────

/// Decode VAPID private key from base64url or PEM format.
fn decode_vapid_private_key(input: &str) -> Result<Vec<u8>, NotiError> {
    // Remove whitespace.
    let filtered: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    // Try base64url decoding.
    if let Ok(decoded) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&filtered) {
        // If it starts with 0x30 (SEQUENCE/DER), it's raw DER.
        if decoded.first() == Some(&0x30) {
            return Ok(decoded);
        }
        // If it's PEM, extract the body.
        if input.contains("-----BEGIN") {
            return extract_pem_body(input);
        }
        // Raw base64 of DER bytes.
        return Ok(decoded);
    }

    // Try PEM format.
    if input.trim().contains("-----BEGIN") {
        return extract_pem_body(input.trim());
    }

    Err(NotiError::Config(
        "invalid VAPID private key: must be base64url or PEM format".into(),
    ))
}

/// Extract base64 body from PEM string.
fn extract_pem_body(input: &str) -> Result<Vec<u8>, NotiError> {
    let b64: String = input
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .flat_map(|l| l.chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&b64)
        .map_err(|e| NotiError::Config(format!("invalid base64 VAPID key: {e}")))
}

/// Base64url encode without padding.
fn b64url_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Generate a VAPID JWT for Web Push authentication.
///
/// VAPID uses ES256 (ECDSA over P-256 with SHA-256) signed JWTs.
fn generate_vapid_jwt(
    vapid_private: &str,
    vapid_email: &str,
    endpoint: &str,
) -> Result<String, NotiError> {
    // Decode the private key.
    let der_bytes = decode_vapid_private_key(vapid_private)?;
    let signing_key = SigningKey::from_pkcs8_der(&der_bytes)
        .map_err(|e| NotiError::Config(format!("failed to parse VAPID private key: {e}")))?;

    // Determine the audience (origin of the push service) from the endpoint.
    let audience = extract_origin(endpoint)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| NotiError::Config(format!("system clock error: {e}")))?;
    let iat = now.as_secs();
    let exp = iat + 86400; // VAPID tokens are valid for 24 hours.

    // Build header JSON: {"alg": "ES256", "typ": "JWT"}.
    let header = json!({
        "alg": "ES256",
        "typ": "JWT"
    });
    let header_json = serde_json::to_string(&header)
        .map_err(|e| NotiError::Config(format!("failed to serialize JWT header: {e}")))?;
    let header_b64 = b64url_encode(header_json.as_bytes());

    // Build claims: {"aud": "<origin>", "exp": <exp>, "iss": "<email>"}.
    let claims = json!({
        "aud": audience,
        "exp": exp,
        "iss": vapid_email
    });
    let claims_json = serde_json::to_string(&claims)
        .map_err(|e| NotiError::Config(format!("failed to serialize JWT claims: {e}")))?;
    let claims_b64 = b64url_encode(claims_json.as_bytes());

    // Sign the signing input (header_b64.claims_b64).
    // JWT ES256 requires raw r||s format (64 bytes), NOT DER encoding.
    let signing_input = format!("{}.{}", header_b64, claims_b64);
    let signature: p256::ecdsa::Signature = signing_key.sign(signing_input.as_bytes());
    // to_bytes() on Signature (not DerSignature) returns the raw 64-byte r||s.
    let sig_b64 = b64url_encode(&signature.to_bytes());

    Ok(format!("{}.{}.{}", header_b64, claims_b64, sig_b64))
}

/// Extract the origin (scheme + host + port) from a push endpoint URL.
fn extract_origin(endpoint: &str) -> Result<String, NotiError> {
    // Parse the endpoint URL manually since we don't have the `url` crate.
    // Expected format: https://push.service.example.com/path or wss://...
    let endpoint_owned = endpoint.to_string();

    // Find the "://" separator.
    let parts: Vec<&str> = endpoint_owned.splitn(2, "://").collect();
    if parts.len() != 2 {
        return Err(NotiError::Config(format!(
            "invalid endpoint URL format (missing scheme): {endpoint}"
        )));
    }
    let scheme = parts[0];
    let rest = parts[1];

    // Find the first "/" or ":" to separate host from path/port.
    let host_end = rest.find(['/', ':']).unwrap_or(rest.len());
    let host = &rest[..host_end];

    if host.is_empty() {
        return Err(NotiError::Config(format!(
            "invalid endpoint URL (missing host): {endpoint}"
        )));
    }

    let port_str = if host_end < rest.len() && rest.chars().nth(host_end) == Some(':') {
        let port_start = host_end + 1;
        let port_end = rest[port_start..].find('/').map(|p| port_start + p).unwrap_or(rest.len());
        let port = &rest[port_start..port_end];
        if port.is_empty() {
            String::new()
        } else {
            format!(":{}", port)
        }
    } else {
        String::new()
    };

    Ok(format!("{}://{}{}", scheme, host, port_str))
}

#[async_trait]
impl NotifyProvider for WebPushProvider {
    fn name(&self) -> &str {
        "webpush"
    }


    fn url_scheme(&self) -> &str {
        "webpush"
    }

    fn description(&self) -> &str {
        "Web Push (VAPID) browser notifications"
    }

    fn example_url(&self) -> &str {
        "webpush://<endpoint_encoded>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("endpoint", "Push subscription endpoint URL"),
            ParamDef::required("p256dh", "Push subscription p256dh key (base64url)"),
            ParamDef::required("auth", "Push subscription auth secret (base64url)"),
            ParamDef::optional("vapid_private", "VAPID private key (base64url or PEM)")
                .with_example("MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcD..."),
            ParamDef::optional("vapid_email", "VAPID contact email")
                .with_example("mailto:admin@example.com"),
            ParamDef::optional("ttl", "Time-to-live in seconds (default: 86400)")
                .with_example("86400"),
            ParamDef::optional(
                "urgency",
                "Push urgency: very-low, low, normal, high (default: normal)",
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
        let endpoint = config.require("endpoint", "webpush")?;

        // Build the notification payload
        let mut notification = json!({
            "body": message.text,
        });

        if let Some(ref title) = message.title {
            notification["title"] = json!(title);
        }

        // Embed first image attachment as data URI in the `image` field
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                notification["image"] = json!(format!("data:{mime};base64,{b64}"));
            }
        } else if message.has_attachments() {
            // For non-image attachments, embed as badge icon
            if let Some(att) = message.attachments.first() {
                if att.kind == AttachmentKind::Image {
                    if let Ok(data) = att.read_bytes().await {
                        let mime = att.effective_mime();
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        notification["badge"] = json!(format!("data:{mime};base64,{b64}"));
                    }
                }
            }
        }

        let payload = serde_json::to_string(&notification).map_err(|e| NotiError::Provider {
            provider: "webpush".into(),
            message: format!("failed to serialize payload: {e}"),
        })?;

        let ttl = config.get("ttl").unwrap_or("86400");
        let urgency = config.get("urgency").unwrap_or("normal");

        // Build the request.
        let mut req_builder = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("TTL", ttl)
            .header("Urgency", urgency);

        // Add VAPID authentication if both private key and email are provided.
        if let (Some(vapid_private), Some(vapid_email)) =
            (config.get("vapid_private"), config.get("vapid_email"))
        {
            let jwt = generate_vapid_jwt(vapid_private, vapid_email, endpoint)?;
            req_builder = req_builder.header("Authorization", format!("vapid t={jwt}"));
        }

        let resp = req_builder
            .body(payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if status == 201 || (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("webpush", "push notification sent via Web Push")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("webpush", format!("Push service error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_origin_https() {
        let result = extract_origin("https://push.example.com/v1/push/abc123").unwrap();
        assert_eq!(result, "https://push.example.com");
    }

    #[test]
    fn test_extract_origin_https_with_port() {
        let result = extract_origin("https://push.example.com:8080/v1/push/abc123").unwrap();
        assert_eq!(result, "https://push.example.com:8080");
    }

    #[test]
    fn test_extract_origin_wss() {
        let result = extract_origin("wss://push.service.example.com/ws/push").unwrap();
        assert_eq!(result, "wss://push.service.example.com");
    }

    #[test]
    fn test_extract_origin_fcm() {
        // Firebase Cloud Messaging endpoint format
        let result = extract_origin("https://fcm.googleapis.com/fcm/send/abc123").unwrap();
        assert_eq!(result, "https://fcm.googleapis.com");
    }

    #[test]
    fn test_extract_origin_mozilla() {
        // Mozilla Push Service endpoint
        let result = extract_origin("https://updates.push.services.mozilla.com/wpush/v1/abc123").unwrap();
        assert_eq!(result, "https://updates.push.services.mozilla.com");
    }

    #[test]
    fn test_extract_origin_missing_scheme() {
        let result = extract_origin("push.example.com/v1/push/abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_origin_empty_host() {
        let result = extract_origin("https:///v1/push");
        assert!(result.is_err());
    }

    #[test]
    fn test_b64url_encode_decode() {
        let original = b"hello world";
        let encoded = b64url_encode(original);
        // URL-safe base64 should not contain + or /
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn test_decode_vapid_private_key_invalid() {
        // Test that invalid keys are handled gracefully
        let invalid_key = "not-a-valid-key";
        let result = decode_vapid_private_key(invalid_key);
        assert!(result.is_err());
    }
}

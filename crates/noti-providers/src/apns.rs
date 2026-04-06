//! Apple Push Notification Service (APNs) provider.
//!
//! Supports token-based authentication using JWT signed with ES256 (ECDSA over P-256 + SHA-256).
//!
//! ### Authentication (token-based, recommended)
//!
//! Requires a p8 private key from the Apple Developer portal:
//! - `key_id` — 10-character key identifier
//! - `team_id` — Apple Developer team identifier
//! - `bundle_id` — app bundle identifier (e.g. `com.example.app`)
//! - `p8_base64` — base64-encoded `.p8` private key content
//!   (-----BEGIN PRIVATE KEY----- ... -----END PRIVATE KEY----- base64-encoded)
//!   OR just the raw base64 bytes without PEM headers)
//! - `p8_path` — path to a `.p8` file on disk (alternative to `p8_base64`)
//!
//! ### Usage
//!
//! ```bash
//! # Via inline base64 key
//! noti send --provider apns \
//!   --param key_id=KEY12345A \
//!   --param team_id=TEAM123456 \
//!   --param bundle_id=com.example.app \
//!   --param p8_base64="$(cat AuthKey_KEY12345A.p8 | base64)" \
//!   --param device_token=abcd1234... \
//!   --message "Hello from APNs!"
//!
//! # Via file path
//! noti send --provider apns \
//!   --param key_id=KEY12345A \
//!   --param team_id=TEAM123456 \
//!   --param bundle_id=com.example.app \
//!   --param p8_path=/path/to/AuthKey_KEY12345A.p8 \
//!   --param device_token=abcd1234... \
//!   --message "Hello from APNs!"
//! ```
//!
//! Reference: <https://developer.apple.com/documentation/usernotifications>

use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use p256::ecdsa::{signature::Signer, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use reqwest::Client;
use serde_json::json;

/// Apple Push Notification Service (APNs) provider.
pub struct ApnsProvider {
    client: Client,
}

impl ApnsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

/// APNs authentication credentials extracted from provider config.
#[derive(Clone)]
struct ApnsCredentials {
    /// DER-encoded PKCS#8 private key bytes.
    pkcs8_der: Vec<u8>,
    /// 10-character APNs key identifier.
    key_id: String,
    /// Apple Developer team identifier.
    team_id: String,
    /// App bundle identifier.
    bundle_id: String,
    /// Use sandbox endpoint (true) or production (false).
    sandbox: bool,
}

impl ApnsCredentials {
    fn from_config(config: &ProviderConfig) -> Result<Self, NotiError> {
        let key_id = config
            .require("key_id", "apns")
            .map_err(|e| NotiError::Config(e.to_string()))?;
        let team_id = config
            .require("team_id", "apns")
            .map_err(|e| NotiError::Config(e.to_string()))?;
        let bundle_id = config
            .require("bundle_id", "apns")
            .map_err(|e| NotiError::Config(e.to_string()))?;

        // Support p8_base64 (inline) or p8_path (file on disk).
        let pkcs8_der = if let Some(base64_input) = config.get("p8_base64") {
            decode_p8_to_der(base64_input)?
        } else if let Some(path) = config.get("p8_path") {
            let content =
                std::fs::read_to_string(path).map_err(|e| {
                    NotiError::Config(format!("failed to read p8 file '{path}': {e}"))
                })?;
            decode_p8_to_der(&content)?
        } else {
            return Err(NotiError::Config(
                "missing required parameter for APNs auth: provide 'p8_base64' \
                 (base64-encoded p8 content) or 'p8_path' (path to .p8 file)"
                    .into(),
            ));
        };

        let sandbox = config
            .get("sandbox")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false);

        Ok(Self {
            pkcs8_der,
            key_id: key_id.to_string(),
            team_id: team_id.to_string(),
            bundle_id: bundle_id.to_string(),
            sandbox,
        })
    }
}

/// Convert p8 content (PEM or base64) to DER-encoded PKCS#8 bytes.
fn decode_p8_to_der(input: &str) -> Result<Vec<u8>, NotiError> {
    // Remove all whitespace.
    let filtered: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    // Extract base64 body from a PEM string (strips -----BEGIN/END----- lines).
    let extract_pem_body = |s: &str| -> Result<Vec<u8>, NotiError> {
        let b64: String = s
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .flat_map(|l| l.chars())
            .filter(|c| !c.is_whitespace())
            .collect();
        base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .map_err(|e| NotiError::Config(format!("invalid base64 p8 content: {e}")))
    };

    // Try base64 decoding the filtered (whitespace-stripped) input.
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&filtered) {
        // If it starts with 0x30 (SEQUENCE), treat as DER.
        if decoded.first() == Some(&0x30) {
            return Ok(decoded);
        }
        // If input had PEM headers, it was base64 of PEM — extract the base64 content.
        if input.contains("-----BEGIN") {
            return extract_pem_body(input);
        }
        // Raw base64 of DER bytes.
        return Ok(decoded);
    }

    // If base64 decode fails, try treating it as a raw PEM string.
    if input.trim().contains("-----BEGIN") {
        return extract_pem_body(input.trim());
    }

    Err(NotiError::Config(
        "p8 content is not valid base64 or PEM".into(),
    ))
}

/// Base64url encode without padding.
fn b64url_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Generate an APNs JWT using ES256.
///
/// Apple requires ES256 (ECDSA over P-256 with SHA-256) signed JWTs.
/// Returns the signed JWT string.
fn generate_apns_jwt(credentials: &ApnsCredentials) -> Result<String, NotiError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| NotiError::Config(format!("system clock error: {e}")))?;
    let iat = now.as_secs();
    let exp = iat + 3600;

    // Build header JSON.
    let header = json!({
        "alg": "ES256",
        "kid": credentials.key_id,
        "typ": "JWT"
    });
    let header_json = serde_json::to_string(&header)
        .map_err(|e| NotiError::Config(format!("failed to serialize JWT header: {e}")))?;
    let header_b64 = b64url_encode(header_json.as_bytes());

    // Build claims JSON.
    let claims = json!({
        "iss": credentials.team_id,
        "iat": iat,
        "exp": exp
    });
    let claims_json = serde_json::to_string(&claims)
        .map_err(|e| NotiError::Config(format!("failed to serialize JWT claims: {e}")))?;
    let claims_b64 = b64url_encode(claims_json.as_bytes());

    // Sign the signing input (header_b64.claims_b64).
    let signing_input = format!("{}.{}", header_b64, claims_b64);
    let signing_bytes = signing_input.as_bytes();

    // Parse the PKCS#8 key and sign with ES256.
    let signing_key = SigningKey::from_pkcs8_der(&credentials.pkcs8_der)
        .map_err(|e| NotiError::Config(format!("failed to parse p8 private key: {e}")))?;

    // Sign with ES256 (returns DER-encoded signature).
    let signature_der: p256::ecdsa::DerSignature = signing_key.sign(signing_bytes);
    let sig_bytes = signature_der.to_bytes();

    // Convert raw r||s bytes (64 bytes for P-256) to DER for JWT.
    // JWT ES256 signatures must be DER-encoded per RFC 7515.
    let sig_der = rs_to_der(&sig_bytes)?;
    let sig_b64 = b64url_encode(&sig_der);

    Ok(format!("{}.{}.{}", header_b64, claims_b64, sig_b64))
}

/// Convert raw r||s bytes (64 bytes for P-256) to DER-encoded ECDSA signature.
///
/// JWT ES256 signatures use DER encoding, not raw r||s concatenation.
/// DER format: SEQUENCE { INTEGER r, INTEGER s }
fn rs_to_der(rs_bytes: &[u8]) -> Result<Vec<u8>, NotiError> {
    if rs_bytes.len() != 64 {
        return Err(NotiError::Config(format!(
            "expected 64-byte r||s signature, got {} bytes",
            rs_bytes.len()
        )));
    }

    let r = &rs_bytes[..32];
    let s = &rs_bytes[32..];

    // Capacity: SEQUENCE(2) + INTEGER r (2..34) + INTEGER s (2..34) ≤ 72.
    let mut der = Vec::with_capacity(72);

    // DER SEQUENCE header: tag 0x30, length (6 + 32 + 32 = 70 = 0x46, but we use length that fits).
    der.push(0x30);
    der.push(0x44); // 68 bytes for the content (2+32 + 2+32 = 68)

    // INTEGER r (32 bytes, leading 0x00 added if high bit set for positive).
    der.push(0x02);
    der.push(0x21); // 33 bytes (add leading 0x00 if r's first byte >= 0x80)
    if r[0] >= 0x80 {
        der.push(0x00);
    }
    der.extend_from_slice(r);

    // INTEGER s.
    der.push(0x02);
    der.push(0x21);
    if s[0] >= 0x80 {
        der.push(0x00);
    }
    der.extend_from_slice(s);

    Ok(der)
}

/// APNs endpoint host.
fn apns_host(sandbox: bool) -> &'static str {
    if sandbox {
        "api.sandbox.push.apple.com"
    } else {
        "api.push.apple.com"
    }
}

/// Build the APNs payload (aps dict) from the message.
async fn build_aps_payload(message: &Message) -> serde_json::Value {
    let mut alert = json!({
        "body": message.text.clone(),
    });

    if let Some(ref title) = message.title {
        alert["title"] = json!(title);
    }

    let mut aps = json!({
        "alert": alert,
    });

    // Badge from extra fields.
    if let Some(badge) = message.extra.get("badge") {
        if let Some(n) = badge.as_u64() {
            aps["badge"] = json!(n);
        }
    }

    // Sound: default or from extra.
    if let Some(sound) = message.extra.get("sound") {
        aps["sound"] = json!(sound);
    } else {
        aps["sound"] = json!("default");
    }

    // Content-available (silent push).
    if message.extra.get("content_available") == Some(&serde_json::json!(1))
        || message.extra.get("content_available") == Some(&serde_json::json!(true))
    {
        aps["content-available"] = json!(1);
    }

    // Category.
    if let Some(cat) = message.extra.get("category") {
        aps["category"] = json!(cat);
    }

    // Thread ID (iOS 12+ grouping).
    if let Some(thread) = message.extra.get("thread_id") {
        aps["thread-id"] = json!(thread);
    }

    // Mutable-content = 1 enables notification service extension.
    aps["mutable-content"] = json!(1);

    // Image attachment as data URI for the extension.
    if let Some(img) = message
        .attachments
        .iter()
        .find(|a| a.kind == AttachmentKind::Image)
    {
        if let Ok(data) = img.read_bytes().await {
            let mime = img.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            aps["image"] = json!(format!("data:{mime};base64,{b64}"));
        }
    }

    aps
}

#[async_trait]
impl NotifyProvider for ApnsProvider {
    fn name(&self) -> &str {
        "apns"
    }

    fn url_scheme(&self) -> &str {
        "apns"
    }

    fn description(&self) -> &str {
        "Apple Push Notification Service (APNs)"
    }

    fn example_url(&self) -> &str {
        "apns://<device_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "key_id",
                "APNs key identifier (10 characters, from Apple Developer portal)",
            )
            .with_example("KEY12345A"),
            ParamDef::required(
                "team_id",
                "Apple Developer Team ID (from Apple Developer portal)",
            )
            .with_example("TEAM1234567"),
            ParamDef::required(
                "bundle_id",
                "iOS app bundle identifier",
            )
            .with_example("com.example.myapp"),
            ParamDef::required(
                "device_token",
                "64-character hex device push token",
            )
            .with_example("abcd1234ef567890..."),
            ParamDef::optional(
                "p8_base64",
                "Base64-encoded .p8 private key (-----BEGIN PRIVATE KEY----- ... -----END PRIVATE KEY----- base64-encoded, or raw base64 bytes without PEM headers)",
            ),
            ParamDef::optional(
                "p8_path",
                "Path to .p8 private key file on disk (alternative to p8_base64)",
            )
            .with_example("/path/to/AuthKey_KEY12345A.p8"),
            ParamDef::optional(
                "sandbox",
                "Use sandbox APNs (true/false, default: false = production)",
            )
            .with_example("false"),
            ParamDef::optional(
                "sound",
                "Notification sound name (default: 'default')",
            )
            .with_example("default"),
            ParamDef::optional(
                "category",
                "Notification category identifier for actionable notifications",
            )
            .with_example("MESSAGE_CATEGORY"),
            ParamDef::optional(
                "thread_id",
                "Thread ID for notification grouping (iOS 12+)",
            )
            .with_example("main-thread"),
            ParamDef::optional(
                "content_available",
                "Enable silent/background push (set to 1, use 'badge' in extra for count)",
            )
            .with_example("0"),
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
        let credentials = ApnsCredentials::from_config(config)?;

        let device_token = config.require("device_token", "apns")?;

        // Validate device token format (64 hex chars).
        if device_token.len() != 64 || !device_token.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NotiError::Validation(format!(
                "invalid device_token: expected 64 hex characters, got {}",
                device_token.len()
            )));
        }

        // Generate JWT.
        let jwt = generate_apns_jwt(&credentials)?;

        let host = apns_host(credentials.sandbox);
        let url = format!("https://{}/3/device/{}", host, device_token);

        // Build payload.
        let aps = build_aps_payload(message).await;
        let payload = json!({
            "aps": aps,
        });

        let resp = self
            .client
            .post(&url)
            .header("authorization", format!("bearer {}", jwt))
            .header("apns-topic", &credentials.bundle_id)
            .header("apns-priority", "10")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();

        if status == 200 {
            let apns_id = resp
                .headers()
                .get("apns-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            Ok(SendResponse::success(
                "apns",
                format!("notification sent via APNs, APNs-ID: {apns_id}"),
            )
            .with_status_code(status))
        } else {
            let raw: serde_json::Value = resp
                .json()
                .await
                .unwrap_or_else(|_| json!({}));

            let reason = raw
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");

            Ok(SendResponse::failure("apns", format!("APNs error ({status}): {reason}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

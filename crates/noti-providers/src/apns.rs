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
    #[derive(Clone, Debug)]
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
    async fn from_config(config: &ProviderConfig) -> Result<Self, NotiError> {
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
        // Reading from disk is done via tokio::fs to avoid blocking the async executor.
        let pkcs8_der = if let Some(base64_input) = config.get("p8_base64") {
            decode_p8_to_der(base64_input)?
        } else if let Some(path) = config.get("p8_path") {
            let content = read_p8_file(path).await?;
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

/// Read a p8 private key file without blocking the async executor.
///
/// Uses tokio::task::spawn_blocking to run the blocking file I/O in a
/// dedicated thread, preventing the async executor from being blocked.
async fn read_p8_file(path: &str) -> Result<String, NotiError> {
    let path_owned = path.to_owned();
    let path_for_error = path_owned.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::read_to_string(&path_owned)
    })
    .await
    .map_err(|e| NotiError::Config(format!("failed to spawn blocking task for p8 file: {e}")))?
    .map_err(|e| NotiError::Config(format!("failed to read p8 file '{path_for_error}': {e}")))
}

/// Convert p8 content (PEM or base64) to DER-encoded PKCS#8 bytes.
fn decode_p8_to_der(input: &str) -> Result<Vec<u8>, NotiError> {
    // Helper: extract base64 body from a PEM string and decode.
    let extract_pem_body = |pem_str: &str| -> Result<Vec<u8>, NotiError> {
        let b64: String = pem_str
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .flat_map(|l| l.chars())
            .filter(|c| !c.is_whitespace())
            .collect();
        base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .map_err(|e| NotiError::Config(format!("invalid base64 p8 content: {e}")))
    };

    // Strip whitespace from input for processing.
    let stripped: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    // Case 1: stripped input contains -----BEGIN marker → input is a PEM string.
    if stripped.contains("-----BEGIN") {
        return extract_pem_body(input);
    }

    // Case 2: stripped input is base64. Try decoding it.
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&stripped) {
        // If decoded content is itself a PEM string (base64-of-PEM was passed),
        // the decoded bytes start with -----BEGIN.
        if decoded.starts_with(b"-----BEGIN") {
            // Extract PEM body from the *decoded* content (the actual PEM string).
            let pem_str = String::from_utf8(decoded)
                .map_err(|_| NotiError::Config("PEM content is not valid UTF-8".into()))?;
            return extract_pem_body(&pem_str);
        }
        // If decoded bytes are raw DER (start with SEQUENCE tag), return as-is.
        if decoded.first() == Some(&0x30) {
            return Ok(decoded);
        }
        // Decoded bytes are raw DER without PEM markers.
        return Ok(decoded);
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

    // Sign with ES256: p256::ecdsa::SigningKey::sign returns DerSignature.
    let der_sig: p256::ecdsa::DerSignature = signing_key.sign(signing_bytes);

    // Extract raw r||s using the Encoding trait (available via ecdsa::der re-export).
    // This gives the 64-byte raw concatenation needed for JWT.
    let raw_rs = der_to_rs(&der_sig.to_bytes())?;
    let jwt_der = rs_to_der(&raw_rs)?;
    let sig_b64 = b64url_encode(&jwt_der);

    Ok(format!("{}.{}.{}", header_b64, claims_b64, sig_b64))
}

/// Parse a DER-encoded ECDSA signature and extract raw r||s bytes (64 bytes for P-256).
///
/// DER format: SEQUENCE { INTEGER r, INTEGER s }
/// Each INTEGER has: tag(1) + length(1) + content(n)
///
/// Returns the raw concatenation of r and s (each 32 bytes, big-endian).
fn der_to_rs(der: &[u8]) -> Result<[u8; 64], NotiError> {
    // Minimum: SEQUENCE(2) + r INTEGER(34) + s INTEGER(34) = 70 bytes
    if der.len() < 70 || der[0] != 0x30 {
        return Err(NotiError::Config(
            "invalid DER signature: expected SEQUENCE tag".into(),
        ));
    }

    let content_len = der[1] as usize;
    let content = &der[2..2 + content_len];

    // Parse INTEGER r: tag(0x02) + length + content
    if content[0] != 0x02 {
        return Err(NotiError::Config(
            "invalid DER signature: expected INTEGER tag for r".into(),
        ));
    }
    let r_len = content[1] as usize; // typically 33 (leading 0x00 + 32 scalar bytes)
    let r_content_start = 2; // byte index within content where r value starts
    let r_scalar = extract_der_scalar(&content[r_content_start..r_content_start + r_len]);

    // Parse INTEGER s: follows r INTEGER in content
    let r_integer_len = 2 + r_len; // tag + length + content
    let s_content_start = r_integer_len;
    if content[s_content_start] != 0x02 {
        return Err(NotiError::Config(
            "invalid DER signature: expected INTEGER tag for s".into(),
        ));
    }
    let s_len = content[s_content_start + 1] as usize;
    let s_scalar = extract_der_scalar(
        &content[s_content_start + 2..s_content_start + 2 + s_len],
    );

    // Combine r || s
    let mut raw = [0u8; 64];
    raw[..32].copy_from_slice(&r_scalar);
    raw[32..].copy_from_slice(&s_scalar);
    Ok(raw)
}

/// Extract a 32-byte scalar from a DER INTEGER's content bytes.
///
/// DER INTEGER content format: [0x00, <31-32 bytes>] or [<32 bytes>]
/// The leading 0x00 (when present) is a DER sign-extension byte.
/// We right-align into a 32-byte array (big-endian for JWT ES256).
fn extract_der_scalar(content: &[u8]) -> [u8; 32] {
    // The DER INTEGER content starts with 0x00 (leading zero if high bit set)
    // followed by the actual scalar bytes. Extract the last 32 bytes.
    let skip = if !content.is_empty() && content[0] == 0x00 { 1 } else { 0 };
    let scalar_len = content.len() - skip;
    let mut scalar = [0u8; 32];
    scalar[32 - scalar_len..].copy_from_slice(&content[skip..]);
    scalar
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

    // Compute DER-encoded length for each INTEGER (32 bytes).
    // Leading 0x00 is added when the high bit is set (to keep integer positive in DER).
    let r_len = if r[0] >= 0x80 { 33 } else { 32 };
    let s_len = if s[0] >= 0x80 { 33 } else { 32 };
    let content_len = 2 + r_len + 2 + s_len; // 2 INTag/length + r + 2 INTag/length + s

    let mut der = Vec::with_capacity(2 + content_len);
    der.push(0x30); // SEQUENCE tag
    der.push(content_len as u8);

    // INTEGER r.
    der.push(0x02);
    der.push(r_len as u8);
    if r[0] >= 0x80 {
        der.push(0x00);
    }
    der.extend_from_slice(r);

    // INTEGER s.
    der.push(0x02);
    der.push(s_len as u8);
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
        let credentials = ApnsCredentials::from_config(config).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // rs_to_der tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_rs_to_der_valid() {
        // Valid 64-byte r||s signature (P-256 produces 64 bytes)
        let rs = vec![0x01; 64];
        let der = rs_to_der(&rs).expect("valid signature should encode");
        // DER SEQUENCE header + r INTEGER + s INTEGER
        assert!(!der.is_empty());
        assert_eq!(der[0], 0x30); // SEQUENCE tag
    }

    #[test]
    fn test_rs_to_der_wrong_length() {
        // Too short (63 bytes)
        let rs = vec![0x01; 63];
        let result = rs_to_der(&rs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("64-byte"));

        // Too long (65 bytes)
        let rs = vec![0x01; 65];
        let result = rs_to_der(&rs);
        assert!(result.is_err());
    }

    #[test]
    fn test_rs_to_der_high_bit_set_adds_leading_zero() {
        // r with high bit set (byte 0 >= 0x80) requires leading 0x00 in DER INTEGER
        // Using a signature where first byte of r is >= 0x80
        let mut rs = vec![0x00; 64];
        rs[0] = 0x80; // Set high bit
        let der = rs_to_der(&rs).expect("valid");
        // After SEQUENCE tag (2 bytes), r INTEGER header is at index 2
        assert_eq!(der[2], 0x02); // INTEGER tag
        // With leading 0x00, length should be 0x21 (33 bytes)
        assert_eq!(der[3], 0x21);
    }

    // -------------------------------------------------------------------------
    // decode_p8_to_der tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_decode_p8_to_der_raw_der() {
        use p256::pkcs8::EncodePrivateKey;
        // Generate a real P-256 key and get its DER bytes
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        // Pass base64-encoded DER (the function detects raw DER via 0x30 prefix
        // after base64 decoding). The function also accepts PEM format.
        let b64_der = base64::engine::general_purpose::STANDARD.encode(&pkcs8_der);
        let result = decode_p8_to_der(&b64_der);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), pkcs8_der);
    }

    #[test]
    fn test_decode_p8_to_der_base64_of_der() {
        use p256::pkcs8::EncodePrivateKey;
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        // base64 of raw DER bytes (no PEM headers)
        let b64 = base64::engine::general_purpose::STANDARD.encode(&pkcs8_der);
        let result = decode_p8_to_der(&b64);
        assert!(result.is_ok(), "base64 of DER should decode: {:?}", result.err());
        assert_eq!(result.unwrap(), pkcs8_der);
    }

    #[test]
    fn test_decode_p8_to_der_base64_of_pem() {
        use p256::pkcs8::EncodePrivateKey;
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        // PEM string → base64 encode → pass as input
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
            base64::engine::general_purpose::STANDARD.encode(&pkcs8_der)
        );
        let b64_of_pem = base64::engine::general_purpose::STANDARD.encode(pem.as_bytes());
        let result = decode_p8_to_der(&b64_of_pem);
        assert!(result.is_ok(), "base64 of PEM should decode: {:?}", result.err());
        assert_eq!(result.unwrap(), pkcs8_der);
    }

    #[test]
    fn test_decode_p8_to_der_raw_pem_string() {
        use p256::pkcs8::EncodePrivateKey;
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        // Raw PEM string (not base64-encoded, just passed directly)
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
            base64::engine::general_purpose::STANDARD.encode(&pkcs8_der)
        );
        let result = decode_p8_to_der(&pem);
        assert!(result.is_ok(), "PEM string should decode: {:?}", result.err());
        assert_eq!(result.unwrap(), pkcs8_der);
    }

    #[test]
    fn test_decode_p8_to_der_with_whitespace() {
        use p256::pkcs8::EncodePrivateKey;
        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        // PEM with extra whitespace (tabs, multiple newlines)
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n\n    {}\n\n-----END PRIVATE KEY-----",
            base64::engine::general_purpose::STANDARD.encode(&pkcs8_der)
        );
        let result = decode_p8_to_der(&pem);
        assert!(result.is_ok(), "PEM with whitespace should decode: {:?}", result.err());
    }

    #[test]
    fn test_decode_p8_to_der_invalid() {
        // Input is not base64 (16 chars, not multiple of 4, so invalid base64)
        let result = decode_p8_to_der("notbase64atall16c");
        assert!(result.is_err(), "invalid base64 should fail");
    }

    // -------------------------------------------------------------------------
    // apns_host tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_apns_host_sandbox() {
        assert_eq!(apns_host(true), "api.sandbox.push.apple.com");
    }

    #[test]
    fn test_apns_host_production() {
        assert_eq!(apns_host(false), "api.push.apple.com");
    }

    // -------------------------------------------------------------------------
    // generate_apns_jwt tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_generate_apns_jwt_format() {
        use p256::pkcs8::EncodePrivateKey;

        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        let credentials = ApnsCredentials {
            pkcs8_der,
            key_id: "KEY12345A".into(),
            team_id: "TEAM123456".into(),
            bundle_id: "com.example.app".into(),
            sandbox: false,
        };

        let jwt = generate_apns_jwt(&credentials).expect("JWT generation should succeed");

        // JWT format: header.payload.signature
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have 3 dot-separated parts");
        assert!(!parts[0].is_empty(), "header must be non-empty");
        assert!(!parts[1].is_empty(), "payload must be non-empty");
        assert!(!parts[2].is_empty(), "signature must be non-empty");
    }

    #[test]
    fn test_generate_apns_jwt_header() {
        use base64::Engine;
        use p256::pkcs8::EncodePrivateKey;

        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        let credentials = ApnsCredentials {
            pkcs8_der,
            key_id: "KEY12345A".into(),
            team_id: "TEAM123456".into(),
            bundle_id: "com.example.app".into(),
            sandbox: false,
        };

        let jwt = generate_apns_jwt(&credentials).unwrap();
        let header_b64 = jwt.split('.').nth(0).unwrap();
        let header_json = String::from_utf8(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(header_b64)
                .expect("valid base64 header")
        ).expect("valid UTF-8 header");

        assert!(header_json.contains("\"alg\":\"ES256\""), "Header must use ES256");
        assert!(header_json.contains("\"kid\":\"KEY12345A\""), "Header must contain key_id");
        assert!(header_json.contains("\"typ\":\"JWT\""), "Header must contain typ JWT");
    }

    #[test]
    fn test_generate_apns_jwt_claims() {
        use base64::Engine;
        use p256::pkcs8::EncodePrivateKey;

        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();

        let credentials = ApnsCredentials {
            pkcs8_der,
            key_id: "KEY12345A".into(),
            team_id: "TEAM123456".into(),
            bundle_id: "com.example.app".into(),
            sandbox: false,
        };

        let jwt = generate_apns_jwt(&credentials).unwrap();
        let claims_b64 = jwt.split('.').nth(1).unwrap();
        let claims_json = String::from_utf8(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(claims_b64)
                .expect("valid base64 claims")
        ).expect("valid UTF-8 claims");

        // iss = team_id
        assert!(claims_json.contains("\"iss\":\"TEAM123456\""), "iss must be team_id");
        // exp = iat + 3600
        assert!(claims_json.contains("\"exp\""), "exp must be present");
        assert!(claims_json.contains("\"iat\""), "iat must be present");
    }

    // -------------------------------------------------------------------------
    // build_aps_payload tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_build_aps_payload_basic() {
        let msg = Message::text("hello world");
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["alert"]["body"], "hello world");
        assert!(payload["alert"]["title"].is_null());
    }

    #[tokio::test]
    async fn test_build_aps_payload_with_title() {
        let msg = Message::text("hello world").with_title("Notification Title");
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["alert"]["title"], "Notification Title");
        assert_eq!(payload["alert"]["body"], "hello world");
    }

    #[tokio::test]
    async fn test_build_aps_payload_badge() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("badge".into(), serde_json::json!(42));
        let msg = Message { text: "test".into(), title: None, format: Default::default(), priority: Default::default(), attachments: vec![], extra };
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["badge"], 42);
    }

    #[tokio::test]
    async fn test_build_aps_payload_sound_default() {
        let msg = Message::text("hello");
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["sound"], "default");
    }

    #[tokio::test]
    async fn test_build_aps_payload_sound_custom() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("sound".into(), serde_json::json!("custom.caf"));
        let msg = Message { text: "test".into(), title: None, format: Default::default(), priority: Default::default(), attachments: vec![], extra };
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["sound"], "custom.caf");
    }

    #[tokio::test]
    async fn test_build_aps_payload_content_available() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("content_available".into(), serde_json::json!(1));
        let msg = Message { text: "test".into(), title: None, format: Default::default(), priority: Default::default(), attachments: vec![], extra };
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["content-available"], 1);
    }

    #[tokio::test]
    async fn test_build_aps_payload_category() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("category".into(), serde_json::json!("MESSAGE_CATEGORY"));
        let msg = Message { text: "test".into(), title: None, format: Default::default(), priority: Default::default(), attachments: vec![], extra };
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["category"], "MESSAGE_CATEGORY");
    }

    #[tokio::test]
    async fn test_build_aps_payload_thread_id() {
        let mut extra = std::collections::HashMap::new();
        extra.insert("thread_id".into(), serde_json::json!("main-thread"));
        let msg = Message { text: "test".into(), title: None, format: Default::default(), priority: Default::default(), attachments: vec![], extra };
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["thread-id"], "main-thread");
    }

    #[tokio::test]
    async fn test_build_aps_payload_mutable_content() {
        let msg = Message::text("hello");
        let payload = build_aps_payload(&msg).await;
        assert_eq!(payload["mutable-content"], 1);
    }

    // -------------------------------------------------------------------------
    // ApnsCredentials::from_config tests (via send validation path)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_apns_credentials_missing_both_p8_params() {
        use noti_core::ProviderConfig;

        let config = ProviderConfig::new()
            .set("key_id", "KEY12345A")
            .set("team_id", "TEAM123456")
            .set("bundle_id", "com.example.app");

        let creds = ApnsCredentials::from_config(&config).await;
        assert!(creds.is_err());
        let err = creds.unwrap_err();
        assert!(err.to_string().contains("p8_base64") || err.to_string().contains("p8_path"));
    }

    #[tokio::test]
    async fn test_apns_credentials_sandbox_flag() {
        use p256::pkcs8::EncodePrivateKey;
        use noti_core::ProviderConfig;

        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let pkcs8_der = signing_key
            .to_pkcs8_der()
            .expect("generate PKCS#8 DER")
            .as_bytes()
            .to_vec();
        let b64_key = base64::engine::general_purpose::STANDARD.encode(&pkcs8_der);

        let config = ProviderConfig::new()
            .set("key_id", "KEY12345A")
            .set("team_id", "TEAM123456")
            .set("bundle_id", "com.example.app")
            .set("p8_base64", &b64_key)
            .set("sandbox", "true");

        let creds = ApnsCredentials::from_config(&config).await;
        assert!(creds.is_ok());
        assert!(creds.unwrap().sandbox);
    }

    // -------------------------------------------------------------------------
    // b64url_encode tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_b64url_encode_no_padding() {
        let data = b"hello world";
        let encoded = b64url_encode(data);
        // URL-safe base64: no + or /
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        // No padding
        assert!(!encoded.contains('='));
    }

    #[test]
    fn test_b64url_encode_known_input() {
        // Test vector: empty string → empty output (with URL-safe no-pad)
        let encoded = b64url_encode(b"");
        assert_eq!(encoded, "");
    }
}

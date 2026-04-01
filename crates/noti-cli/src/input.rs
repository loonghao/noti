use anyhow::{Result, bail};

/// Validate that a string input does not contain control characters (ASCII < 0x20),
/// except for common whitespace (\n, \r, \t).
///
/// AI agents may hallucinate invisible control characters that break downstream
/// processing. This enforces a "zero trust" model for agent-generated inputs.
pub fn reject_control_chars(value: &str, field_name: &str) -> Result<()> {
    for (i, ch) in value.chars().enumerate() {
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            bail!(
                "invalid control character (U+{:04X}) at position {i} in {field_name}; \
                 control characters below ASCII 0x20 are not allowed",
                ch as u32
            );
        }
    }
    Ok(())
}

/// Validate that a file path does not contain path traversal sequences.
///
/// Agents may hallucinate `../../` segments that escape the intended directory.
/// This rejects any path containing `..` components.
pub fn reject_path_traversal(path: &str, field_name: &str) -> Result<()> {
    let normalized = path.replace('\\', "/");
    if normalized.contains("../") || normalized.contains("/..") || normalized == ".." {
        bail!(
            "path traversal detected in {field_name}: '{path}'; \
             paths containing '..' are not allowed for security"
        );
    }
    Ok(())
}

/// Validate that a resource identifier does not contain embedded query parameters.
///
/// Agents sometimes hallucinate query strings inside resource IDs
/// (e.g. `fileId?fields=name`). The CLI rejects `?` and `#` in identifiers.
pub fn reject_embedded_query_params(value: &str, field_name: &str) -> Result<()> {
    if value.contains('?') || value.contains('#') {
        bail!(
            "invalid characters in {field_name}: '{value}'; \
             resource identifiers must not contain '?' or '#'"
        );
    }
    Ok(())
}

/// Run all input hardening checks on a general string field.
pub fn validate_string_input(value: &str, field_name: &str) -> Result<()> {
    reject_control_chars(value, field_name)?;
    Ok(())
}

/// Run all input hardening checks on a file path field.
pub fn validate_file_path(path: &str, field_name: &str) -> Result<()> {
    reject_control_chars(path, field_name)?;
    reject_path_traversal(path, field_name)?;
    Ok(())
}

/// Run all input hardening checks on a resource identifier field.
pub fn validate_identifier(value: &str, field_name: &str) -> Result<()> {
    reject_control_chars(value, field_name)?;
    reject_embedded_query_params(value, field_name)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_control_chars_ok() {
        assert!(reject_control_chars("hello world", "test").is_ok());
        assert!(reject_control_chars("line\nnewline", "test").is_ok());
        assert!(reject_control_chars("tab\there", "test").is_ok());
        assert!(reject_control_chars("cr\rhere", "test").is_ok());
    }

    #[test]
    fn test_reject_control_chars_rejects_null() {
        assert!(reject_control_chars("hello\0world", "test").is_err());
    }

    #[test]
    fn test_reject_control_chars_rejects_bell() {
        assert!(reject_control_chars("hello\x07world", "test").is_err());
    }

    #[test]
    fn test_reject_control_chars_rejects_escape() {
        assert!(reject_control_chars("hello\x1bworld", "test").is_err());
    }

    #[test]
    fn test_reject_path_traversal_ok() {
        assert!(reject_path_traversal("file.txt", "test").is_ok());
        assert!(reject_path_traversal("dir/file.txt", "test").is_ok());
        assert!(reject_path_traversal("/absolute/path.txt", "test").is_ok());
        assert!(reject_path_traversal("./relative/path.txt", "test").is_ok());
    }

    #[test]
    fn test_reject_path_traversal_rejects_dotdot() {
        assert!(reject_path_traversal("../secret", "test").is_err());
        assert!(reject_path_traversal("dir/../etc/passwd", "test").is_err());
        assert!(reject_path_traversal("..\\windows\\path", "test").is_err());
        assert!(reject_path_traversal("..", "test").is_err());
    }

    #[test]
    fn test_reject_embedded_query_params_ok() {
        assert!(reject_embedded_query_params("webhook-key-123", "test").is_ok());
        assert!(reject_embedded_query_params("slack-token", "test").is_ok());
    }

    #[test]
    fn test_reject_embedded_query_params_rejects_question() {
        assert!(reject_embedded_query_params("key?fields=name", "test").is_err());
    }

    #[test]
    fn test_reject_embedded_query_params_rejects_hash() {
        assert!(reject_embedded_query_params("key#anchor", "test").is_err());
    }

    #[test]
    fn test_validate_file_path_comprehensive() {
        assert!(validate_file_path("normal/path.txt", "test").is_ok());
        assert!(validate_file_path("../escape", "test").is_err());
        assert!(validate_file_path("path\0null", "test").is_err());
    }

    #[test]
    fn test_validate_identifier_comprehensive() {
        assert!(validate_identifier("valid-id-123", "test").is_ok());
        assert!(validate_identifier("id?query", "test").is_err());
        assert!(validate_identifier("id\0null", "test").is_err());
    }
}

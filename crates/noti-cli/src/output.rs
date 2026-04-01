use serde::Serialize;
use std::collections::HashSet;

/// Whether to output in JSON or human-readable format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

impl OutputMode {
    /// Detect output mode from `--json` flag or `NOTI_OUTPUT` env var.
    ///
    /// Priority: explicit `--json` flag > `NOTI_OUTPUT=json` env var > Human.
    /// This allows agents to set the env var once instead of passing `--json` on every call.
    pub fn detect(json_flag: bool) -> Self {
        if json_flag {
            return Self::Json;
        }
        if let Ok(val) = std::env::var("NOTI_OUTPUT") {
            if val.eq_ignore_ascii_case("json") {
                return Self::Json;
            }
        }
        Self::Human
    }
}

/// Print a success message.
pub fn print_success(mode: OutputMode, message: &str) {
    match mode {
        OutputMode::Json => {
            let out = serde_json::json!({ "status": "success", "message": message });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
        }
        OutputMode::Human => {
            println!("✓ {message}");
        }
    }
}

/// Print an error message with structured JSON for agents.
pub fn print_error(mode: OutputMode, message: &str) {
    match mode {
        OutputMode::Json => {
            let out = serde_json::json!({
                "status": "error",
                "code": 1,
                "message": message,
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
            );
        }
        OutputMode::Human => {
            eprintln!("✗ {message}");
        }
    }
}

/// Print a serializable value as JSON, optionally filtering to specified fields.
pub fn print_json_filtered<T: Serialize>(value: &T, fields: &Option<Vec<String>>) {
    let json_value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    let output = match fields {
        Some(field_list) if !field_list.is_empty() => filter_fields(&json_value, field_list),
        _ => json_value,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}

/// Print a serializable value as JSON (always, no field filtering).
pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}

/// Filter a JSON value to only include the specified fields.
///
/// For objects, only top-level keys matching the field list are kept.
/// For arrays, each element is filtered individually.
fn filter_fields(value: &serde_json::Value, fields: &[String]) -> serde_json::Value {
    let field_set: HashSet<&str> = fields.iter().map(|s| s.as_str()).collect();

    match value {
        serde_json::Value::Object(map) => {
            let filtered: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .filter(|(k, _)| field_set.contains(k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            serde_json::Value::Object(filtered)
        }
        serde_json::Value::Array(arr) => {
            let filtered: Vec<serde_json::Value> =
                arr.iter().map(|v| filter_fields(v, fields)).collect();
            serde_json::Value::Array(filtered)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_detect_flag() {
        assert_eq!(OutputMode::detect(true), OutputMode::Json);
        assert_eq!(OutputMode::detect(false), OutputMode::Human);
    }

    #[test]
    fn test_filter_fields_object() {
        let val = serde_json::json!({
            "name": "test",
            "provider": "slack",
            "message": "hello",
            "raw_response": {"key": "value"}
        });
        let fields = vec!["name".into(), "provider".into()];
        let filtered = filter_fields(&val, &fields);
        assert_eq!(
            filtered,
            serde_json::json!({"name": "test", "provider": "slack"})
        );
    }

    #[test]
    fn test_filter_fields_array() {
        let val = serde_json::json!([
            {"name": "a", "extra": 1},
            {"name": "b", "extra": 2},
        ]);
        let fields = vec!["name".into()];
        let filtered = filter_fields(&val, &fields);
        assert_eq!(
            filtered,
            serde_json::json!([{"name": "a"}, {"name": "b"}])
        );
    }

    #[test]
    fn test_filter_fields_scalar_passthrough() {
        let val = serde_json::json!("hello");
        let fields = vec!["name".into()];
        let filtered = filter_fields(&val, &fields);
        assert_eq!(filtered, serde_json::json!("hello"));
    }
}

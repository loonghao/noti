use serde::Serialize;

/// Whether to output in JSON or human-readable format.
#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Human,
    Json,
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

/// Print an error message.
pub fn print_error(mode: OutputMode, message: &str) {
    match mode {
        OutputMode::Json => {
            let out = serde_json::json!({ "status": "error", "message": message });
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

/// Print a serializable value as JSON (always).
pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}

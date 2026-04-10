//! Shared test utilities for provider send tests.

pub use noti_core::{Message, MessageFormat, NotifyProvider, ProviderConfig};
pub use reqwest::Client;
pub use wiremock::matchers::{header, method, path};
pub use wiremock::{Mock, MockServer, ResponseTemplate};
pub use url::Url;

/// Create a shared HTTP client for tests.
pub fn client() -> Client {
    Client::new()
}

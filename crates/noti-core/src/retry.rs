use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::NotiError;
use crate::message::Message;
use crate::provider::{NotifyProvider, ProviderConfig, SendResponse};

/// Configuration for retry behavior when sending notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,
    /// Initial delay before the first retry.
    #[serde(with = "duration_millis")]
    pub initial_delay: Duration,
    /// Maximum delay between retries (caps exponential backoff).
    #[serde(with = "duration_millis")]
    pub max_delay: Duration,
    /// Backoff multiplier applied after each retry.
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create a policy that never retries.
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create a policy with a fixed delay (no exponential backoff).
    pub fn fixed(max_retries: u32, delay: Duration) -> Self {
        Self {
            max_retries,
            initial_delay: delay,
            max_delay: delay,
            backoff_multiplier: 1.0,
        }
    }

    /// Create an exponential backoff policy.
    pub fn exponential(max_retries: u32, initial_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay,
            backoff_multiplier: 2.0,
        }
    }

    /// Calculate the delay for a given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi((attempt - 1) as i32);
        let delay = Duration::from_millis(delay_ms.min(self.max_delay.as_millis() as f64) as u64);
        delay.min(self.max_delay)
    }

    /// Whether the given attempt (0-indexed) should be retried.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Outcome of executing a send operation with retry logic.
#[derive(Debug, Clone)]
pub struct RetryOutcome<T> {
    /// The final result (success or last error).
    pub result: T,
    /// Number of attempts made (1 = succeeded on first try).
    pub attempts: u32,
    /// Total time spent across all attempts (if tracked externally).
    pub total_duration: Option<Duration>,
}

/// Send a message through a provider with automatic retry according to the given policy.
///
/// On transient failures (network errors, provider errors with retryable status codes),
/// the function will retry up to `policy.max_retries` times with exponential backoff.
/// Validation errors are never retried since they indicate a permanent problem.
pub async fn send_with_retry(
    provider: &dyn NotifyProvider,
    message: &Message,
    config: &ProviderConfig,
    policy: &RetryPolicy,
) -> RetryOutcome<Result<SendResponse, NotiError>> {
    let start = Instant::now();
    let mut attempt = 0u32;

    loop {
        match provider.send(message, config).await {
            Ok(response) => {
                return RetryOutcome {
                    result: Ok(response),
                    attempts: attempt + 1,
                    total_duration: Some(start.elapsed()),
                };
            }
            Err(err) => {
                if !is_retryable(&err) || !policy.should_retry(attempt) {
                    return RetryOutcome {
                        result: Err(err),
                        attempts: attempt + 1,
                        total_duration: Some(start.elapsed()),
                    };
                }
                attempt += 1;
                let delay = policy.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Execute an arbitrary async closure with retry logic.
///
/// This is a generic helper that can be used for any fallible async operation,
/// not just provider sends.
pub async fn execute_with_retry<F, Fut, T>(
    policy: &RetryPolicy,
    mut operation: F,
) -> RetryOutcome<Result<T, NotiError>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, NotiError>>,
{
    let start = Instant::now();
    let mut attempt = 0u32;

    loop {
        match operation().await {
            Ok(value) => {
                return RetryOutcome {
                    result: Ok(value),
                    attempts: attempt + 1,
                    total_duration: Some(start.elapsed()),
                };
            }
            Err(err) => {
                if !is_retryable(&err) || !policy.should_retry(attempt) {
                    return RetryOutcome {
                        result: Err(err),
                        attempts: attempt + 1,
                        total_duration: Some(start.elapsed()),
                    };
                }
                attempt += 1;
                let delay = policy.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// Determine whether an error is transient and worth retrying.
///
/// Validation errors are never retried (they indicate permanent problems).
/// Network and provider errors are considered retryable.
fn is_retryable(err: &NotiError) -> bool {
    match err {
        NotiError::Network(_) => true,
        NotiError::Provider { .. } => true,
        NotiError::Io(_) => true,
        NotiError::Validation(_) | NotiError::Config(_) | NotiError::UrlParse(_) => false,
    }
}

/// Serde helper: serialize/deserialize `Duration` as milliseconds.
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ParamDef, SendResponse};
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_default_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_none_policy() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.max_retries, 0);
        assert!(!policy.should_retry(0));
    }

    #[test]
    fn test_fixed_policy() {
        let policy = RetryPolicy::fixed(5, Duration::from_secs(2));
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(2));
    }

    #[test]
    fn test_exponential_delays() {
        let policy = RetryPolicy::exponential(5, Duration::from_secs(1), Duration::from_secs(16));

        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(8));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(16));
        // Capped at max_delay
        assert_eq!(policy.delay_for_attempt(6), Duration::from_secs(16));
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::default(); // max_retries = 3
        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[test]
    fn test_serde_roundtrip() {
        let policy = RetryPolicy::default();
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: RetryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_retries, policy.max_retries);
        assert_eq!(parsed.initial_delay, policy.initial_delay);
        assert_eq!(parsed.max_delay, policy.max_delay);
    }

    #[test]
    fn test_is_retryable_network() {
        assert!(is_retryable(&NotiError::Network("timeout".into())));
    }

    #[test]
    fn test_is_retryable_provider() {
        assert!(is_retryable(&NotiError::provider("slack", "500")));
    }

    #[test]
    fn test_is_not_retryable_validation() {
        assert!(!is_retryable(&NotiError::Validation("bad param".into())));
    }

    #[test]
    fn test_is_not_retryable_config() {
        assert!(!is_retryable(&NotiError::Config("bad config".into())));
    }

    #[test]
    fn test_is_not_retryable_url_parse() {
        assert!(!is_retryable(&NotiError::UrlParse("bad url".into())));
    }

    // --- Mock provider for async tests ---

    struct MockProvider {
        fail_count: AtomicU32,
        call_count: Arc<AtomicU32>,
    }

    impl MockProvider {
        fn new(fail_first_n: u32) -> Self {
            Self {
                fail_count: AtomicU32::new(fail_first_n),
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl NotifyProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn url_scheme(&self) -> &str {
            "mock"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "mock provider"
        }
        fn example_url(&self) -> &str {
            "mock://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let remaining = self.fail_count.load(Ordering::SeqCst);
            if remaining > 0 {
                self.fail_count.fetch_sub(1, Ordering::SeqCst);
                Err(NotiError::Network("simulated transient error".into()))
            } else {
                Ok(SendResponse::success("mock", "ok"))
            }
        }
    }

    /// A mock that always returns a validation error (non-retryable).
    struct ValidationErrorProvider;

    #[async_trait]
    impl NotifyProvider for ValidationErrorProvider {
        fn name(&self) -> &str {
            "validation-mock"
        }
        fn url_scheme(&self) -> &str {
            "validation-mock"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "always fails with validation"
        }
        fn example_url(&self) -> &str {
            "validation-mock://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            Err(NotiError::Validation("permanent error".into()))
        }
    }

    #[tokio::test]
    async fn test_send_with_retry_success_first_try() {
        let provider = MockProvider::new(0);
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(1));

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.attempts, 1);
        assert_eq!(provider.calls(), 1);
    }

    #[tokio::test]
    async fn test_send_with_retry_succeeds_after_retries() {
        let provider = MockProvider::new(2); // fail first 2, succeed on 3rd
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(1));

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.attempts, 3);
        assert_eq!(provider.calls(), 3);
    }

    #[tokio::test]
    async fn test_send_with_retry_exhausts_retries() {
        let provider = MockProvider::new(10); // always fail
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::fixed(2, Duration::from_millis(1));

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.result.is_err());
        // 1 initial + 2 retries = 3 attempts
        assert_eq!(outcome.attempts, 3);
        assert_eq!(provider.calls(), 3);
    }

    #[tokio::test]
    async fn test_send_with_retry_no_retry_on_validation() {
        let provider = ValidationErrorProvider;
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(1));

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.result.is_err());
        assert_eq!(outcome.attempts, 1); // no retries for validation errors
    }

    #[tokio::test]
    async fn test_send_with_retry_none_policy() {
        let provider = MockProvider::new(1); // fail once
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::none();

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.result.is_err());
        assert_eq!(outcome.attempts, 1);
    }

    #[tokio::test]
    async fn test_send_with_retry_tracks_duration() {
        let provider = MockProvider::new(0);
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let policy = RetryPolicy::none();

        let outcome = send_with_retry(&provider, &msg, &config, &policy).await;
        assert!(outcome.total_duration.is_some());
    }

    #[tokio::test]
    async fn test_execute_with_retry_success() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(1));

        let outcome = execute_with_retry(&policy, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, NotiError>("done".to_string())
            }
        })
        .await;

        assert!(outcome.result.is_ok());
        assert_eq!(outcome.attempts, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_execute_with_retry_retries_then_succeeds() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let policy = RetryPolicy::fixed(3, Duration::from_millis(1));

        let outcome = execute_with_retry(&policy, || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(NotiError::Network("transient".into()))
                } else {
                    Ok::<_, NotiError>(42u32)
                }
            }
        })
        .await;

        assert!(outcome.result.is_ok());
        assert_eq!(outcome.result.unwrap(), 42);
        assert_eq!(outcome.attempts, 3);
    }
}

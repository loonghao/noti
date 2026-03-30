use std::time::Instant;

use crate::error::NotiError;
use crate::message::Message;
use crate::provider::{NotifyProvider, ProviderConfig, SendResponse};
use crate::retry::{RetryOutcome, RetryPolicy, send_with_retry};

/// A target for batch sending: a provider paired with its configuration.
pub struct SendTarget<'a> {
    /// The notification provider to send through.
    pub provider: &'a dyn NotifyProvider,
    /// Provider-specific configuration.
    pub config: &'a ProviderConfig,
}

impl<'a> SendTarget<'a> {
    /// Create a new send target.
    pub fn new(provider: &'a dyn NotifyProvider, config: &'a ProviderConfig) -> Self {
        Self { provider, config }
    }
}

/// Result of sending a message to a single target within a batch.
#[derive(Debug)]
pub struct TargetResult {
    /// Name of the provider.
    pub provider_name: String,
    /// The retry outcome (includes attempts count and duration).
    pub outcome: RetryOutcome<Result<SendResponse, NotiError>>,
}

impl TargetResult {
    /// Whether this individual send succeeded.
    pub fn is_success(&self) -> bool {
        self.outcome
            .result
            .as_ref()
            .map(|r| r.success)
            .unwrap_or(false)
    }
}

/// Result of a batch send operation across multiple targets.
#[derive(Debug)]
pub struct BatchResult {
    /// Per-target results, in the same order as the input targets.
    pub results: Vec<TargetResult>,
    /// Total wall-clock time for the entire batch.
    pub total_duration: std::time::Duration,
}

impl BatchResult {
    /// Number of targets that succeeded.
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_success()).count()
    }

    /// Number of targets that failed.
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| !r.is_success()).count()
    }

    /// Whether all targets succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| r.is_success())
    }

    /// Whether any target succeeded.
    pub fn any_succeeded(&self) -> bool {
        self.results.iter().any(|r| r.is_success())
    }
}

/// Send a message to multiple targets in parallel, each with retry.
///
/// All targets are dispatched concurrently using `tokio::join!` semantics.
/// Each target independently applies the given retry policy.
pub async fn send_batch(
    targets: &[SendTarget<'_>],
    message: &Message,
    policy: &RetryPolicy,
) -> BatchResult {
    let start = Instant::now();

    let futures: Vec<_> = targets
        .iter()
        .map(|target| async {
            let name = target.provider.name().to_string();
            let outcome = send_with_retry(target.provider, message, target.config, policy).await;
            TargetResult {
                provider_name: name,
                outcome,
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    BatchResult {
        results,
        total_duration: start.elapsed(),
    }
}

/// Send a message to the first target that succeeds (failover mode).
///
/// Tries each target in order. If a target fails after retries, moves to the next.
/// Returns as soon as one target succeeds, or after all have been tried.
pub async fn send_failover(
    targets: &[SendTarget<'_>],
    message: &Message,
    policy: &RetryPolicy,
) -> BatchResult {
    let start = Instant::now();
    let mut results = Vec::with_capacity(targets.len());

    for target in targets {
        let name = target.provider.name().to_string();
        let outcome = send_with_retry(target.provider, message, target.config, policy).await;
        let success = outcome
            .result
            .as_ref()
            .map(|r| r.success)
            .unwrap_or(false);
        results.push(TargetResult {
            provider_name: name,
            outcome,
        });
        if success {
            break;
        }
    }

    BatchResult {
        results,
        total_duration: start.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ParamDef;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    struct SuccessProvider {
        name: String,
    }

    impl SuccessProvider {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl NotifyProvider for SuccessProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn url_scheme(&self) -> &str {
            "success"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "always succeeds"
        }
        fn example_url(&self) -> &str {
            "success://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            Ok(SendResponse::success(&self.name, "ok"))
        }
    }

    struct FailProvider {
        name: String,
        call_count: AtomicU32,
    }

    impl FailProvider {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                call_count: AtomicU32::new(0),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl NotifyProvider for FailProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn url_scheme(&self) -> &str {
            "fail"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "always fails"
        }
        fn example_url(&self) -> &str {
            "fail://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Err(NotiError::Network("always fails".into()))
        }
    }

    #[tokio::test]
    async fn test_send_batch_all_succeed() {
        let p1 = SuccessProvider::new("p1");
        let p2 = SuccessProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1, &config),
            SendTarget::new(&p2, &config),
        ];

        let result = send_batch(&targets, &msg, &policy).await;
        assert!(result.all_succeeded());
        assert_eq!(result.success_count(), 2);
        assert_eq!(result.failure_count(), 0);
        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn test_send_batch_partial_failure() {
        let p1 = SuccessProvider::new("p1");
        let p2 = FailProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1, &config),
            SendTarget::new(&p2 as &dyn NotifyProvider, &config),
        ];

        let result = send_batch(&targets, &msg, &policy).await;
        assert!(!result.all_succeeded());
        assert!(result.any_succeeded());
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 1);
    }

    #[tokio::test]
    async fn test_send_batch_all_fail() {
        let p1 = FailProvider::new("p1");
        let p2 = FailProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1 as &dyn NotifyProvider, &config),
            SendTarget::new(&p2 as &dyn NotifyProvider, &config),
        ];

        let result = send_batch(&targets, &msg, &policy).await;
        assert!(!result.all_succeeded());
        assert!(!result.any_succeeded());
        assert_eq!(result.failure_count(), 2);
    }

    #[tokio::test]
    async fn test_send_batch_empty_targets() {
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();
        let targets: Vec<SendTarget> = vec![];

        let result = send_batch(&targets, &msg, &policy).await;
        assert!(result.all_succeeded()); // vacuous truth
        assert_eq!(result.results.len(), 0);
    }

    #[tokio::test]
    async fn test_send_batch_with_retry() {
        let p1 = FailProvider::new("p1");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::fixed(2, Duration::from_millis(1));

        let targets = vec![SendTarget::new(&p1 as &dyn NotifyProvider, &config)];

        let result = send_batch(&targets, &msg, &policy).await;
        assert_eq!(result.failure_count(), 1);
        // 1 initial + 2 retries = 3 calls
        assert_eq!(p1.calls(), 3);
        assert_eq!(result.results[0].outcome.attempts, 3);
    }

    #[tokio::test]
    async fn test_send_failover_first_succeeds() {
        let p1 = SuccessProvider::new("p1");
        let p2 = FailProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1, &config),
            SendTarget::new(&p2 as &dyn NotifyProvider, &config),
        ];

        let result = send_failover(&targets, &msg, &policy).await;
        assert!(result.any_succeeded());
        // Only first target was tried (it succeeded)
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].provider_name, "p1");
        assert_eq!(p2.calls(), 0);
    }

    #[tokio::test]
    async fn test_send_failover_falls_to_second() {
        let p1 = FailProvider::new("p1");
        let p2 = SuccessProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1 as &dyn NotifyProvider, &config),
            SendTarget::new(&p2, &config),
        ];

        let result = send_failover(&targets, &msg, &policy).await;
        assert!(result.any_succeeded());
        assert_eq!(result.results.len(), 2);
        assert!(!result.results[0].is_success());
        assert!(result.results[1].is_success());
    }

    #[tokio::test]
    async fn test_send_failover_all_fail() {
        let p1 = FailProvider::new("p1");
        let p2 = FailProvider::new("p2");
        let config = ProviderConfig::new();
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();

        let targets = vec![
            SendTarget::new(&p1 as &dyn NotifyProvider, &config),
            SendTarget::new(&p2 as &dyn NotifyProvider, &config),
        ];

        let result = send_failover(&targets, &msg, &policy).await;
        assert!(!result.any_succeeded());
        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn test_batch_result_tracks_duration() {
        let msg = Message::text("hello");
        let policy = RetryPolicy::none();
        let targets: Vec<SendTarget> = vec![];

        let result = send_batch(&targets, &msg, &policy).await;
        assert!(result.total_duration < Duration::from_secs(1));
    }
}

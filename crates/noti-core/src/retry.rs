use std::time::Duration;

use serde::{Deserialize, Serialize};

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
        let policy = RetryPolicy::exponential(
            5,
            Duration::from_secs(1),
            Duration::from_secs(16),
        );

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
}

use std::fmt::Debug;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A clock for providing current time in milliseconds since Unix epoch.
///
/// This trait enables time abstraction for `CircuitBreaker`, allowing
/// deterministic testing without real-time sleeps.
pub trait Clock: Send + Sync {
    /// Returns the current time in milliseconds since Unix epoch.
    fn now_ms(&self) -> u64;
}

/// System time clock using `SystemTime::now()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Mock clock for deterministic testing.
///
/// Advances time programmatically via `advance()` instead of relying
/// on real-time sleeps.
///
/// Internally uses `Arc<AtomicU64>` so that cloning shares the same time state.
/// This allows a single `MockClock` to be shared between test code and the
/// `CircuitBreaker` under test.
#[cfg(test)]
pub struct MockClock {
    now_ms: Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(test)]
impl MockClock {
    /// Create a new mock clock starting at the given time (ms since epoch).
    pub fn new(start_ms: u64) -> Self {
        Self {
            now_ms: Arc::new(std::sync::atomic::AtomicU64::new(start_ms)),
        }
    }

    /// Advance the mock clock by the given duration.
    #[allow(dead_code)]
    pub fn advance(&self, duration: Duration) {
        self.now_ms
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
    }

    /// Set the mock clock to a specific time.
    #[allow(dead_code)]
    pub fn set(&self, ms: u64) {
        self.now_ms.store(ms, Ordering::SeqCst);
    }
}

#[cfg(test)]
impl Clone for MockClock {
    fn clone(&self) -> Self {
        Self {
            now_ms: Arc::clone(&self.now_ms),
        }
    }
}

#[cfg(test)]
impl Debug for MockClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockClock")
            .field("now_ms", &self.now_ms.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
impl Clock for MockClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

/// Configuration for circuit breaker behavior.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u32,
    /// Number of consecutive successes needed to close the circuit from half-open.
    pub success_threshold: u32,
    /// Time to wait before transitioning from Open to HalfOpen.
    pub open_duration: Duration,
    /// Maximum consecutive successes in Closed state before resetting failure count.
    pub reset_success_count: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            open_duration: Duration::from_secs(30),
            reset_success_count: 10,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a strict circuit breaker (opens quickly, requires more success to close).
    pub fn strict() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 3,
            open_duration: Duration::from_secs(60),
            reset_success_count: 5,
        }
    }

    /// Create a lenient circuit breaker (requires more failures to open, shorter open duration).
    pub fn lenient() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 1,
            open_duration: Duration::from_secs(10),
            reset_success_count: 20,
        }
    }
}

/// State of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed — requests go through normally.
    Closed,
    /// Circuit is open — requests fail fast without calling the provider.
    Open,
    /// Circuit is half-open — testing if provider has recovered.
    HalfOpen,
}

/// Core circuit breaker state and logic.
///
/// Uses atomics for lock-free updates. All operations are O(1).
///
/// The `C` type parameter is the clock used for time tracking.
pub struct CircuitBreaker<C: Clock = SystemClock> {
    config: CircuitBreakerConfig,
    clock: C,
    /// Consecutive failure count (resets on success when closed).
    failures: AtomicU32,
    /// Consecutive success count (resets on failure when closed).
    successes: AtomicU32,
    /// Timestamp when the circuit was opened.
    opened_at: std::sync::atomic::AtomicU64,
    /// Current state (stored in a u32 for atomic operations).
    state: std::sync::atomic::AtomicU32,
}

const STATE_CLOSED: u32 = 0;
const STATE_OPEN: u32 = 1;
const STATE_HALF_OPEN: u32 = 2;

impl<C: Clock> CircuitBreaker<C> {
    /// Create a new circuit breaker with a custom configuration and explicit clock.
    ///
    /// This constructor is useful for tests that need to control time.
    pub fn with_config_and_clock(config: CircuitBreakerConfig, clock: C) -> Self {
        Self {
            config,
            clock,
            failures: AtomicU32::new(0),
            successes: AtomicU32::new(0),
            opened_at: std::sync::atomic::AtomicU64::new(0),
            state: std::sync::atomic::AtomicU32::new(STATE_CLOSED),
        }
    }
}

impl CircuitBreaker<SystemClock> {
    /// Create a new circuit breaker with the default configuration and system clock.
    pub fn new() -> Self {
        Self::with_config_and_clock(CircuitBreakerConfig::default(), SystemClock)
    }

    /// Create a new circuit breaker with a custom configuration and system clock.
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self::with_config_and_clock(config, SystemClock)
    }
}

impl<C: Clock> CircuitBreaker<C> {
    /// Record a successful provider call.
    pub fn record_success(&self) {
        let current_state = self.state.load(Ordering::SeqCst);

        match current_state {
            STATE_CLOSED => {
                let successes = self.successes.fetch_add(1, Ordering::SeqCst) + 1;
                // Reset failure count on success
                self.failures.store(0, Ordering::SeqCst);
                // If we've had many consecutive successes, optionally reset the counter
                // (this is a soft reset to prevent the failure counter from becoming stale)
                if successes >= self.config.reset_success_count {
                    self.successes.store(0, Ordering::SeqCst);
                }
            }
            STATE_HALF_OPEN => {
                let successes = self.successes.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.success_threshold {
                    // Enough successes in half-open — close the circuit
                    self.state.store(STATE_CLOSED, Ordering::SeqCst);
                    self.failures.store(0, Ordering::SeqCst);
                    self.successes.store(0, Ordering::SeqCst);
                    self.opened_at.store(0, Ordering::SeqCst);
                }
            }
            STATE_OPEN => {
                // Should not happen — record_success should not be called when open
            }
            _ => {
                // Invalid state — reset to closed
                self.force_state(CircuitState::Closed);
            }
        }
    }

    /// Record a failed provider call.
    pub fn record_failure(&self) {
        let current_state = self.state.load(Ordering::SeqCst);

        match current_state {
            STATE_CLOSED => {
                let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
                // Reset success count on failure
                self.successes.store(0, Ordering::SeqCst);
                if failures >= self.config.failure_threshold {
                    // Open the circuit
                    self.state.store(STATE_OPEN, Ordering::SeqCst);
                    self.opened_at.store(self.clock.now_ms(), Ordering::SeqCst);
                }
            }
            STATE_HALF_OPEN => {
                // Any failure in half-open reopens the circuit immediately
                self.state.store(STATE_OPEN, Ordering::SeqCst);
                self.successes.store(0, Ordering::SeqCst);
                self.failures
                    .fetch_add(1, Ordering::SeqCst); // Count this as a failure for threshold
                self.opened_at.store(self.clock.now_ms(), Ordering::SeqCst);
            }
            STATE_OPEN => {
                // Already open — count failures but don't change state
                self.failures.fetch_add(1, Ordering::SeqCst);
            }
            _ => {
                // Invalid state — reset to closed and record failure
                self.force_state(CircuitState::Closed);
                self.failures.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    /// Returns true if the circuit is open and requests should fail fast.
    pub fn is_open(&self) -> bool {
        let current_state = self.state.load(Ordering::SeqCst);

        if current_state == STATE_CLOSED {
            return false;
        }

        if current_state == STATE_OPEN {
            // Check if we should transition to half-open
            let opened_at_ms = self.opened_at.load(Ordering::SeqCst);
            let now_ms = self.clock.now_ms();
            let open_duration_ms = self.config.open_duration.as_millis() as u64;
            if now_ms >= opened_at_ms + open_duration_ms {
                // Transition to half-open
                self.state.store(STATE_HALF_OPEN, Ordering::SeqCst);
                self.successes.store(0, Ordering::SeqCst);
                return false;
            }
            return true;
        }

        // Half-open — allow request through to test
        false
    }

    /// Returns the current state of the circuit.
    pub fn state(&self) -> CircuitState {
        let s = self.state.load(Ordering::SeqCst);
        match s {
            STATE_CLOSED => CircuitState::Closed,
            STATE_OPEN => {
                // Check if we should transition to half-open
                let opened_at_ms = self.opened_at.load(Ordering::SeqCst);
                let now_ms = self.clock.now_ms();
                let open_duration_ms = self.config.open_duration.as_millis() as u64;
                if now_ms >= opened_at_ms + open_duration_ms {
                    // Transition to half-open
                    self.state.store(STATE_HALF_OPEN, Ordering::SeqCst);
                    self.successes.store(0, Ordering::SeqCst);
                    return CircuitState::HalfOpen;
                }
                CircuitState::Open
            }
            STATE_HALF_OPEN => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    /// Returns the current failure count.
    pub fn failure_count(&self) -> u32 {
        self.failures.load(Ordering::SeqCst)
    }

    /// Returns the current success count.
    pub fn success_count(&self) -> u32 {
        self.successes.load(Ordering::SeqCst)
    }

    /// Force the circuit into a specific state (useful for testing and manual overrides).
    pub fn force_state(&self, state: CircuitState) {
        match state {
            CircuitState::Closed => {
                self.state.store(STATE_CLOSED, Ordering::SeqCst);
                self.failures.store(0, Ordering::SeqCst);
                self.successes.store(0, Ordering::SeqCst);
                self.opened_at.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {
                self.state.store(STATE_OPEN, Ordering::SeqCst);
                self.opened_at.store(self.clock.now_ms(), Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                self.state.store(STATE_HALF_OPEN, Ordering::SeqCst);
                self.successes.store(0, Ordering::SeqCst);
            }
        }
    }
}

impl Default for CircuitBreaker<SystemClock> {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared circuit breaker that can be used across multiple tasks.
pub type SharedCircuitBreaker = Arc<CircuitBreaker<SystemClock>>;

/// A registry of circuit breakers, one per provider.
#[derive(Default)]
pub struct CircuitBreakerRegistry {
    breakers: std::sync::RwLock<std::collections::HashMap<String, SharedCircuitBreaker>>,
}

impl CircuitBreakerRegistry {
    /// Create a new circuit breaker registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a circuit breaker for the given provider.
    pub fn get_or_create(&self, provider_name: &str) -> SharedCircuitBreaker {
        // Fast path: try to get existing breaker
        {
            let readers = self.breakers.read().unwrap();
            if let Some(cb) = readers.get(provider_name) {
                return cb.clone();
            }
        }

        // Slow path: create new breaker
        let new_breaker = Arc::new(CircuitBreaker::new());
        let mut writers = self.breakers.write().unwrap();
        if let Some(cb) = writers.get(provider_name) {
            return cb.clone();
        }
        writers.insert(provider_name.to_string(), new_breaker.clone());
        new_breaker
    }

    /// Get the circuit breaker for a provider if it exists.
    pub fn get(&self, provider_name: &str) -> Option<SharedCircuitBreaker> {
        let readers = self.breakers.read().unwrap();
        readers.get(provider_name).cloned()
    }

    /// Remove a circuit breaker for a provider.
    pub fn remove(&self, provider_name: &str) {
        let mut writers = self.breakers.write().unwrap();
        writers.remove(provider_name);
    }

    /// Clear all circuit breakers.
    pub fn clear(&self) {
        let mut writers = self.breakers.write().unwrap();
        writers.clear();
    }

    /// Returns the number of registered breakers.
    pub fn len(&self) -> usize {
        let readers = self.breakers.read().unwrap();
        readers.len()
    }

    /// Returns true if there are no registered breakers.
    pub fn is_empty(&self) -> bool {
        let readers = self.breakers.read().unwrap();
        readers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_resets_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_half_open_after_duration() {
        // Deterministic test using MockClock instead of thread::sleep
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_duration: Duration::from_millis(50),
            ..Default::default()
        };
        let clock = MockClock::new(0);
        let cb = CircuitBreaker::with_config_and_clock(config, clock.clone());

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());

        // Advance clock past open_duration
        clock.advance(Duration::from_millis(100));

        // Should transition to half-open on next check via is_open()
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_closes_on_success_in_half_open() {
        // Deterministic test using MockClock instead of thread::sleep
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_duration: Duration::from_millis(10),
            ..Default::default()
        };
        let clock = MockClock::new(0);
        let cb = CircuitBreaker::with_config_and_clock(config, clock.clone());

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Advance clock past open_duration to trigger half-open transition
        clock.advance(Duration::from_millis(50));

        // Transition via is_open()
        assert!(!cb.is_open());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Success in half-open closes the circuit
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_reopens_on_failure_in_half_open() {
        // Deterministic test using MockClock instead of thread::sleep
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_duration: Duration::from_millis(10),
            ..Default::default()
        };
        let clock = MockClock::new(0);
        let cb = CircuitBreaker::with_config_and_clock(config, clock.clone());

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Advance clock past open_duration to trigger half-open transition
        clock.advance(Duration::from_millis(50));

        // Transition via is_open()
        assert!(!cb.is_open());

        // Failure in half-open reopens
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_config_lenient() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig::lenient());
        assert_eq!(cb.config.failure_threshold, 10);
        assert_eq!(cb.config.success_threshold, 1);
    }

    #[test]
    fn test_circuit_config_strict() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig::strict());
        assert_eq!(cb.config.failure_threshold, 3);
        assert_eq!(cb.config.success_threshold, 3);
    }

    #[test]
    fn test_force_state() {
        let cb = CircuitBreaker::new();

        cb.force_state(CircuitState::Open);
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());

        cb.force_state(CircuitState::HalfOpen);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(!cb.is_open());

        cb.force_state(CircuitState::Closed);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_registry_get_or_create() {
        let registry = CircuitBreakerRegistry::new();

        let cb1 = registry.get_or_create("slack");
        let cb2 = registry.get_or_create("slack");
        let cb3 = registry.get_or_create("email");

        assert!(Arc::ptr_eq(&cb1, &cb2));
        assert!(!Arc::ptr_eq(&cb1, &cb3));
    }

    #[test]
    fn test_registry_remove() {
        let registry = CircuitBreakerRegistry::new();

        registry.get_or_create("slack");
        assert_eq!(registry.len(), 1);

        registry.remove("slack");
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_clear() {
        let registry = CircuitBreakerRegistry::new();

        registry.get_or_create("slack");
        registry.get_or_create("email");
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let config = CircuitBreakerConfig {
            failure_threshold: 100,
            ..Default::default()
        };
        let cb = Arc::new(CircuitBreaker::with_config(config));

        let mut handles = vec![];
        for _ in 0..10 {
            let cb = cb.clone();
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    cb.record_failure();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 10 threads * 10 failures = 100 failures
        assert_eq!(cb.failure_count(), 100);
        assert_eq!(cb.state(), CircuitState::Open);
    }
}

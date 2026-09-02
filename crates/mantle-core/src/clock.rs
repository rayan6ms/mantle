use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Monotonic time source used by cleanup and stuck detection.
pub trait Clock: Clone {
    fn now(&self) -> Duration;
}

/// Production monotonic clock, measured from construction.
#[derive(Clone, Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Cloneable deterministic clock for tests and embedding-controlled virtual time.
#[derive(Clone, Debug, Default)]
pub struct ManualClock {
    nanos: Arc<AtomicU64>,
}

impl ManualClock {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&self, duration: Duration) {
        let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        let _ = self
            .nanos
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_add(nanos))
            });
    }

    pub fn set(&self, time: Duration) {
        let nanos = u64::try_from(time.as_nanos()).unwrap_or(u64::MAX);
        let _ = self
            .nanos
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.max(nanos))
            });
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_is_shared_and_monotonic_when_advanced() {
        let clock = ManualClock::new();
        let clone = clock.clone();
        clock.advance(Duration::from_millis(25));
        assert_eq!(clone.now(), Duration::from_millis(25));
        clone.advance(Duration::from_millis(5));
        assert_eq!(clock.now(), Duration::from_millis(30));
        clock.set(Duration::from_millis(10));
        assert_eq!(clock.now(), Duration::from_millis(30));
        clock.set(Duration::MAX);
        clock.advance(Duration::from_nanos(1));
        assert_eq!(clock.now(), Duration::from_nanos(u64::MAX));
    }
}

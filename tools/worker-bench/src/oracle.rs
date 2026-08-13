use std::fmt;
use std::time::Duration;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DeliveryCounts {
    pub frames_requested: u64,
    pub frames_delivered: u64,
    pub frame_underruns: u64,
    pub skipped_deadlines: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationWidthError {
    expected: usize,
    actual: usize,
}

impl fmt::Display for ObservationWidthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "delivery tick expected {} track observations but received {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for ObservationWidthError {}

/// A deterministic delivery oracle driven by logical ticks rather than wall-clock sleeps.
pub struct DeliveryOracle {
    tracks: usize,
    counts: DeliveryCounts,
}

impl DeliveryOracle {
    #[must_use]
    pub const fn new(tracks: usize) -> Self {
        Self {
            tracks,
            counts: DeliveryCounts {
                frames_requested: 0,
                frames_delivered: 0,
                frame_underruns: 0,
                skipped_deadlines: 0,
            },
        }
    }

    pub fn observe_tick<I>(&mut self, observations: I) -> Result<(), ObservationWidthError>
    where
        I: IntoIterator<Item = bool>,
    {
        let mut actual = 0_usize;
        let mut delivered = 0_u64;
        for observation in observations {
            actual = actual.saturating_add(1);
            delivered = delivered.saturating_add(u64::from(observation));
        }
        if actual != self.tracks {
            return Err(ObservationWidthError {
                expected: self.tracks,
                actual,
            });
        }
        let requested = u64::try_from(actual).unwrap_or(u64::MAX);
        self.counts.frames_requested = self.counts.frames_requested.saturating_add(requested);
        self.counts.frames_delivered = self.counts.frames_delivered.saturating_add(delivered);
        self.counts.frame_underruns = self
            .counts
            .frame_underruns
            .saturating_add(requested.saturating_sub(delivered));
        Ok(())
    }

    pub fn observe_lateness(&mut self, lateness: Duration, period: Duration) {
        self.counts.skipped_deadlines = self
            .counts
            .skipped_deadlines
            .saturating_add(skipped_periods(lateness, period));
    }

    #[must_use]
    pub const fn counts(&self) -> DeliveryCounts {
        self.counts
    }
}

#[must_use]
pub fn skipped_periods(lateness: Duration, period: Duration) -> u64 {
    if period.is_zero() {
        return 0;
    }
    u64::try_from(lateness.as_nanos() / period.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{DeliveryCounts, DeliveryOracle, skipped_periods};

    #[test]
    fn all_delivered_ticks_are_counted_exactly() {
        let mut oracle = DeliveryOracle::new(3);
        oracle.observe_tick([true, true, true]).unwrap();
        oracle.observe_tick([true, true, true]).unwrap();
        assert_eq!(
            oracle.counts(),
            DeliveryCounts {
                frames_requested: 6,
                frames_delivered: 6,
                frame_underruns: 0,
                skipped_deadlines: 0,
            }
        );
    }

    #[test]
    fn empty_reads_are_one_underrun_per_track_and_tick() {
        let mut oracle = DeliveryOracle::new(4);
        oracle.observe_tick([true, false, false, true]).unwrap();
        assert_eq!(oracle.counts().frames_requested, 4);
        assert_eq!(oracle.counts().frames_delivered, 2);
        assert_eq!(oracle.counts().frame_underruns, 2);
    }

    #[test]
    fn wrong_tick_width_is_rejected_without_mutating_counts() {
        let mut oracle = DeliveryOracle::new(2);
        assert!(oracle.observe_tick([true]).is_err());
        assert_eq!(oracle.counts(), DeliveryCounts::default());
    }

    #[test]
    fn skipped_deadlines_count_only_whole_periods() {
        let period = Duration::from_millis(20);
        assert_eq!(skipped_periods(Duration::from_millis(19), period), 0);
        assert_eq!(skipped_periods(Duration::from_millis(20), period), 1);
        assert_eq!(skipped_periods(Duration::from_millis(59), period), 2);
    }
}

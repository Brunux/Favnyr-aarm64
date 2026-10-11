//! Bounded throughput sampling for one copy or move operation.

use std::collections::VecDeque;
use std::time::Duration;

const WINDOW: Duration = Duration::from_secs(3);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
const WARMUP: Duration = Duration::from_millis(500);

#[derive(Debug, PartialEq, Eq)]
pub struct TransferEstimate {
    pub bytes_per_second: u64,
    pub remaining_seconds: Option<u64>,
}

pub struct TransferMeter {
    transferred: u64,
    samples: VecDeque<(Duration, u64)>,
}

impl TransferMeter {
    /// The caller's elapsed clock starts after scanning the source sizes.
    pub fn new() -> Self {
        Self {
            transferred: 0,
            samples: VecDeque::from([(Duration::ZERO, 0)]),
        }
    }

    /// Only bytes reported by the copy callback count towards throughput.
    /// A successful filesystem rename advances progress without transferring data.
    pub fn add_bytes(&mut self, delta: u64) {
        self.transferred = self.transferred.saturating_add(delta);
    }

    pub fn estimate(
        &mut self,
        elapsed: Duration,
        remaining_bytes: u64,
    ) -> Option<TransferEstimate> {
        let cutoff = elapsed.saturating_sub(WINDOW);
        // Keep one sample at or before the window boundary to measure the
        // intervening bytes. A long gap is included instead of inventing a burst.
        while self.samples.len() > 1 && self.samples[1].0 <= cutoff {
            self.samples.pop_front();
        }
        let (since, baseline) = self.samples[0];
        let interval = elapsed.saturating_sub(since);
        let bytes = self.transferred.saturating_sub(baseline);
        if elapsed.saturating_sub(self.samples.back()?.0) >= SAMPLE_INTERVAL {
            self.samples.push_back((elapsed, self.transferred));
        }
        if interval < WARMUP || bytes == 0 {
            return None;
        }
        let rate = bytes as f64 / interval.as_secs_f64();
        Some(TransferEstimate {
            bytes_per_second: (rate as u64).max(1),
            remaining_seconds: (remaining_bytes > 0)
                .then(|| (remaining_bytes as f64 / rate).ceil() as u64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_waits_for_a_measurable_interval() {
        let mut meter = TransferMeter::new();
        meter.add_bytes(500);
        assert_eq!(meter.estimate(Duration::from_millis(100), 500), None);
        assert_eq!(
            meter.estimate(Duration::from_millis(500), 501),
            Some(TransferEstimate {
                bytes_per_second: 1000,
                remaining_seconds: Some(1),
            })
        );
    }

    #[test]
    fn rate_adapts_to_recent_transfer_speed() {
        let mut meter = TransferMeter::new();
        for second in 1..=3 {
            meter.add_bytes(1000);
            meter.estimate(Duration::from_secs(second), 30_000);
        }
        for second in 4..=6 {
            meter.add_bytes(4000);
            meter.estimate(Duration::from_secs(second), 30_000);
        }
        assert_eq!(
            meter.estimate(Duration::from_secs(6), 10_001),
            Some(TransferEstimate {
                bytes_per_second: 4000,
                remaining_seconds: Some(3),
            })
        );
    }

    #[test]
    fn rename_only_progress_has_no_transfer_estimate() {
        let mut meter = TransferMeter::new();
        assert_eq!(meter.estimate(Duration::from_secs(1), 1_000_000), None);
        assert_eq!(meter.estimate(Duration::from_secs(5), 0), None);
    }

    #[test]
    fn a_pause_reduces_the_rate_and_expands_the_estimate() {
        let mut meter = TransferMeter::new();
        meter.add_bytes(1000);
        assert_eq!(
            meter.estimate(Duration::from_secs(1), 2000),
            Some(TransferEstimate {
                bytes_per_second: 1000,
                remaining_seconds: Some(2),
            })
        );
        meter.add_bytes(1000);
        assert_eq!(
            meter.estimate(Duration::from_secs(6), 2000),
            Some(TransferEstimate {
                bytes_per_second: 200,
                remaining_seconds: Some(10),
            })
        );
    }

    #[test]
    fn completed_bytes_keep_the_rate_without_claiming_remaining_work() {
        let mut meter = TransferMeter::new();
        meter.add_bytes(1024);
        assert_eq!(
            meter.estimate(Duration::from_secs(1), 0),
            Some(TransferEstimate {
                bytes_per_second: 1024,
                remaining_seconds: None,
            })
        );
    }

    #[test]
    fn frequent_callbacks_keep_a_bounded_history() {
        let mut meter = TransferMeter::new();
        for millisecond in 1..=10_000 {
            meter.add_bytes(1);
            meter.estimate(Duration::from_millis(millisecond), 500);
        }
        assert!(meter.samples.len() <= 32);
        assert_eq!(
            meter.estimate(Duration::from_secs(10), 500),
            Some(TransferEstimate {
                bytes_per_second: 1000,
                remaining_seconds: Some(1),
            })
        );
    }
}

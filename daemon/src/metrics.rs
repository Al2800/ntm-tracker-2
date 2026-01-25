//! Timing metrics and performance instrumentation.
//!
//! This module provides utilities for tracking timing metrics across the daemon.
//! All metrics are exposed through tracing spans and can be aggregated by log analysis tools.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Global metrics collector.
pub static METRICS: Metrics = Metrics::new();

/// Timing histogram bucket for latency tracking.
#[derive(Debug, Default)]
pub struct Histogram {
    count: AtomicU64,
    sum_us: AtomicU64,
    min_us: AtomicU64,
    max_us: AtomicU64,
}

impl Histogram {
    pub const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            sum_us: AtomicU64::new(0),
            min_us: AtomicU64::new(u64::MAX),
            max_us: AtomicU64::new(0),
        }
    }

    /// Record a duration.
    pub fn record(&self, duration: Duration) {
        let us = duration.as_micros() as u64;
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_us.fetch_add(us, Ordering::Relaxed);

        // Update min (using compare-and-swap loop)
        let mut current_min = self.min_us.load(Ordering::Relaxed);
        while us < current_min {
            match self.min_us.compare_exchange_weak(
                current_min,
                us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_min) => current_min = new_min,
            }
        }

        // Update max
        let mut current_max = self.max_us.load(Ordering::Relaxed);
        while us > current_max {
            match self.max_us.compare_exchange_weak(
                current_max,
                us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => current_max = new_max,
            }
        }
    }

    /// Get histogram statistics.
    pub fn stats(&self) -> HistogramStats {
        let count = self.count.load(Ordering::Relaxed);
        let sum_us = self.sum_us.load(Ordering::Relaxed);
        let min_us = self.min_us.load(Ordering::Relaxed);
        let max_us = self.max_us.load(Ordering::Relaxed);

        HistogramStats {
            count,
            sum_us,
            min_us: if min_us == u64::MAX { 0 } else { min_us },
            max_us,
            avg_us: if count > 0 { sum_us / count } else { 0 },
        }
    }

    /// Reset the histogram.
    pub fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.sum_us.store(0, Ordering::Relaxed);
        self.min_us.store(u64::MAX, Ordering::Relaxed);
        self.max_us.store(0, Ordering::Relaxed);
    }
}

/// Histogram statistics snapshot.
#[derive(Debug, Clone, Default)]
pub struct HistogramStats {
    pub count: u64,
    pub sum_us: u64,
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: u64,
}

/// Global metrics storage.
pub struct Metrics {
    /// tmux command execution times
    pub tmux_cmd: Histogram,
    /// ntm command execution times
    pub ntm_cmd: Histogram,
    /// Polling cycle durations
    pub poll_cycle: Histogram,
    /// Event processing latency
    pub event_processing: Histogram,
    /// DB write latency
    pub db_write: Histogram,
    /// RPC request handling time
    pub rpc_request: Histogram,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub const fn new() -> Self {
        Self {
            tmux_cmd: Histogram::new(),
            ntm_cmd: Histogram::new(),
            poll_cycle: Histogram::new(),
            event_processing: Histogram::new(),
            db_write: Histogram::new(),
            rpc_request: Histogram::new(),
        }
    }

    /// Get all metrics as a summary.
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            tmux_cmd: self.tmux_cmd.stats(),
            ntm_cmd: self.ntm_cmd.stats(),
            poll_cycle: self.poll_cycle.stats(),
            event_processing: self.event_processing.stats(),
            db_write: self.db_write.stats(),
            rpc_request: self.rpc_request.stats(),
        }
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.tmux_cmd.reset();
        self.ntm_cmd.reset();
        self.poll_cycle.reset();
        self.event_processing.reset();
        self.db_write.reset();
        self.rpc_request.reset();
    }
}

/// Summary of all metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsSummary {
    pub tmux_cmd: HistogramStats,
    pub ntm_cmd: HistogramStats,
    pub poll_cycle: HistogramStats,
    pub event_processing: HistogramStats,
    pub db_write: HistogramStats,
    pub rpc_request: HistogramStats,
}

/// RAII timer that records duration on drop.
pub struct Timer<'a> {
    histogram: &'a Histogram,
    start: Instant,
}

impl<'a> Timer<'a> {
    pub fn new(histogram: &'a Histogram) -> Self {
        Self {
            histogram,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        self.histogram.record(self.start.elapsed());
    }
}

/// Convenience macro for timing a block.
#[macro_export]
macro_rules! time_it {
    ($histogram:expr, $block:expr) => {{
        let _timer = $crate::metrics::Timer::new($histogram);
        $block
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_records_values() {
        let h = Histogram::new();
        h.record(Duration::from_micros(100));
        h.record(Duration::from_micros(200));
        h.record(Duration::from_micros(150));

        let stats = h.stats();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_us, 100);
        assert_eq!(stats.max_us, 200);
        assert_eq!(stats.sum_us, 450);
        assert_eq!(stats.avg_us, 150);
    }

    #[test]
    fn histogram_reset_clears_values() {
        let h = Histogram::new();
        h.record(Duration::from_micros(100));
        h.reset();

        let stats = h.stats();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.min_us, 0);
        assert_eq!(stats.max_us, 0);
    }

    #[test]
    fn timer_records_on_drop() {
        let h = Histogram::new();
        {
            let _timer = Timer::new(&h);
            std::thread::sleep(Duration::from_millis(1));
        }
        let stats = h.stats();
        assert_eq!(stats.count, 1);
        assert!(stats.min_us >= 1000); // At least 1ms
    }

    #[test]
    fn global_metrics_work() {
        METRICS.tmux_cmd.record(Duration::from_micros(500));
        let summary = METRICS.summary();
        assert!(summary.tmux_cmd.count >= 1);
    }
}

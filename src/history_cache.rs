//! In-memory cache for historical power samples
//!
//! Provides a rolling window of recent power data for fast retrieval.

use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::Duration;
use log::debug;
use crate::poller::PowerSample;

/// Unix timestamp in seconds
pub type UnixTs = u64;

/// In-memory rolling cache for the most recent samples
///
/// Intended for fast range queries over the last 48 hours.
pub struct HistoryCache {
    inner: RwLock<VecDeque<PowerSample>>,
    retention_secs: u64,
}

impl HistoryCache {
    /// Create an empty cache with the given retention window in seconds
    ///
    /// # Arguments
    ///
    /// * `retention_secs` - the number of seconds to keep samples in the cache
    pub fn new(retention_secs: u64) -> Self {
        Self {
            inner: RwLock::new(VecDeque::new()),
            retention_secs,
        }
    }

    /// Insert one sample and prune anything older than the retention window
    ///
    /// # Arguments
    ///
    /// * `sample` - the power sample to insert
    pub fn insert(&self, sample: PowerSample) {
        let mut guard = self.inner.write().unwrap();
        guard.push_back(sample);
        self.prune_locked(&mut guard, sample.ts);
        debug!("inserted sample, cache size: {}", guard.len());
    }


    /// Query samples in the inclusive range `[from_ts, to_ts]`
    ///
    /// # Arguments
    ///
    /// * `from_ts` - the start of the range (inclusive)
    /// * `to_ts` - the end of the range (inclusive)
    pub fn query(&self, from_ts: UnixTs, to_ts: UnixTs) -> Vec<PowerSample> {
        if from_ts > to_ts {
            return Vec::new();
        }

        let guard = self.inner.read().unwrap();
        guard
            .iter()
            .copied()
            .filter(|sample| sample.ts >= from_ts && sample.ts <= to_ts)
            .collect()
    }

    /// Prunes samples older than the retention window from the queue
    ///
    /// # Arguments
    ///
    /// * `queue` - the queue to prune
    /// * `now_ts` - the current timestamp in seconds
    fn prune_locked(&self, queue: &mut VecDeque<PowerSample>, now_ts: u64) {
        let cutoff = now_ts.saturating_sub(self.retention_secs);

        while let Some(front) = queue.front() {
            if front.ts < cutoff {
                queue.pop_front();
            } else {
                break;
            }
        }
    }
}

/// Energy represented during one time bucket.
///
/// Values are in kWh for the period ending at `ts`.
#[derive(Debug, Copy, Clone)]
pub struct PowerDelta {
    /// Unix timestamp in seconds, normally the end of the bucket
    pub ts: UnixTs,
    /// Energy produced during this bucket in kWh
    pub produced: f64,
    /// Energy consumed during this bucket in kWh
    pub consumed: f64,
    /// Energy exported during this bucket in kWh
    pub exported: f64,
}

/// Convert absolute cumulative samples into per-period deltas.
///
/// `bucket_size` defines the wanted period, for example:
/// - `Duration::from_secs(60)` for 1 minute
/// - `Duration::from_secs(5 * 60)` for 5 minutes
/// - `Duration::from_secs(15 * 60)` for 15 minutes
/// - `Duration::from_secs(60 * 60)` for 1 hour
///
/// Samples are grouped by truncated bucket start. The first bucket is used as
/// the baseline. Each following bucket returns the difference between the last
/// sample in that bucket and the last sample in the previous bucket.
///
/// For example, with 5-minute buckets:
///
/// - a sample at `16:34:58` belongs to bucket `16:30:00`
/// - a sample at `16:39:58` belongs to bucket `16:35:00`
///
/// The returned delta for the second bucket gets `ts = 16:35:00`.
pub fn power_deltas(samples: &[PowerSample], bucket_size: Duration) -> Vec<PowerDelta> {
    let bucket_secs = bucket_size.as_secs();

    if samples.len() < 2 || bucket_secs == 0 {
        return Vec::new();
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by_key(|sample| sample.ts);

    let mut result = Vec::new();

    let mut previous_bucket_start = align_to_bucket(sorted[0].ts, bucket_secs);
    let mut previous_bucket_last = sorted[0];

    let mut current_bucket_start = previous_bucket_start;
    let mut current_bucket_last = previous_bucket_last;

    for sample in sorted.into_iter().skip(1) {
        let sample_bucket_start = align_to_bucket(sample.ts, bucket_secs);

        if sample_bucket_start == current_bucket_start {
            current_bucket_last = sample;
            continue;
        }

        if current_bucket_start != previous_bucket_start {
            if let Some(mut delta) = delta_between(previous_bucket_last, current_bucket_last) {
                delta.ts = current_bucket_start;
                result.push(delta);
            }

            previous_bucket_last = current_bucket_last;
            previous_bucket_start = current_bucket_start;
        } else {
            previous_bucket_last = current_bucket_last;
            previous_bucket_start = current_bucket_start;
        }

        current_bucket_start = sample_bucket_start;
        current_bucket_last = sample;
    }

    if current_bucket_start != previous_bucket_start {
        if let Some(mut delta) = delta_between(previous_bucket_last, current_bucket_last) {
            delta.ts = current_bucket_start;
            result.push(delta);
        }
    }

    result
}

fn align_to_bucket(ts: UnixTs, bucket_secs: u64) -> UnixTs {
    ts - ts % bucket_secs
}

fn delta_between(previous: PowerSample, current: PowerSample) -> Option<PowerDelta> {
    Some(PowerDelta {
        ts: current.ts,
        produced: checked_energy_delta(previous.produced, current.produced)?,
        consumed: checked_energy_delta(previous.consumed, current.consumed)?,
        exported: checked_energy_delta(previous.exported, current.exported)?,
    })
}
fn checked_energy_delta(previous: u32, current: u32) -> Option<f64> {
    if current < previous {
        return None;
    }

    Some((current - previous) as f64 * 0.1)
}
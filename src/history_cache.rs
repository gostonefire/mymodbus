//! In-memory cache for historical power samples
//!
//! Provides a rolling window of recent power data for fast retrieval.

use std::collections::VecDeque;
use std::sync::RwLock;
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

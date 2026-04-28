use std::collections::VecDeque;
use std::sync::RwLock;
use log::debug;
use crate::poller::PowerSample;

pub type UnixTs = u64;

/// In-memory rolling cache for the most recent samples.
///
/// Intended for fast range queries over the last 48 hours.
pub struct HistoryCache {
    inner: RwLock<VecDeque<PowerSample>>,
    retention_secs: u64,
}

impl HistoryCache {
    /// Create an empty cache with the given retention window in seconds.
    pub fn new(retention_secs: u64) -> Self {
        Self {
            inner: RwLock::new(VecDeque::new()),
            retention_secs,
        }
    }

    /// Insert one sample and prune anything older than the retention window.
    pub fn insert(&self, sample: PowerSample) {
        let mut guard = self.inner.write().unwrap();
        guard.push_back(sample);
        self.prune_locked(&mut guard, sample.ts);
        debug!("inserted sample, cache size: {}", guard.len());
    }


    /// Query samples in the inclusive range `[from_ts, to_ts]`.
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

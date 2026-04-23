use std::collections::VecDeque;
use std::sync::RwLock;

use crate::persistence::{unix_now_secs, PowerSample};

pub type UnixTs = i64;

/// In-memory rolling cache for the most recent samples.
///
/// Intended for fast range queries over the last 48 hours.
pub struct HistoryCache {
    inner: RwLock<VecDeque<PowerSample>>,
    retention_secs: i64,
}

impl HistoryCache {
    /// Create an empty cache with the given retention window in seconds.
    pub fn new(retention_secs: i64) -> Self {
        Self {
            inner: RwLock::new(VecDeque::new()),
            retention_secs,
        }
    }

    /// Create a cache from existing samples and immediately prune older data.
    pub fn from_samples(retention_secs: i64, samples: impl IntoIterator<Item = PowerSample>) -> Self {
        let cache = Self::new(retention_secs);
        {
            let mut guard = cache.inner.write().unwrap();
            for sample in samples {
                guard.push_back(sample);
            }
            cache.prune_locked(&mut guard, unix_now_secs());
        }
        cache
    }

    /// Insert one sample and prune anything older than the retention window.
    pub fn insert(&self, sample: PowerSample) {
        let mut guard = self.inner.write().unwrap();
        guard.push_back(sample);
        self.prune_locked(&mut guard, sample.ts);
    }

    /// Insert many samples efficiently.
    pub fn extend<I>(&self, samples: I)
    where
        I: IntoIterator<Item = PowerSample>,
    {
        let mut guard = self.inner.write().unwrap();
        let mut newest_ts: Option<i64> = None;

        for sample in samples {
            newest_ts = Some(match newest_ts {
                Some(current) => current.max(sample.ts),
                None => sample.ts,
            });
            guard.push_back(sample);
        }

        if let Some(ts) = newest_ts {
            self.prune_locked(&mut guard, ts);
        }
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

    /// Return the newest sample, if any.
    pub fn latest(&self) -> Option<PowerSample> {
        let guard = self.inner.read().unwrap();
        guard.back().copied()
    }

    /// Return the number of samples currently held in memory.
    pub fn len(&self) -> usize {
        let guard = self.inner.read().unwrap();
        guard.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove all samples from memory.
    pub fn clear(&self) {
        let mut guard = self.inner.write().unwrap();
        guard.clear();
    }

    /// Return the configured retention window in seconds.
    pub fn retention_secs(&self) -> i64 {
        self.retention_secs
    }

    fn prune_locked(&self, queue: &mut VecDeque<PowerSample>, now_ts: i64) {
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

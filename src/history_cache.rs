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

    /// Return the configured retention window in seconds.
    /// 
    pub fn retention_secs(&self) -> u64 {
        self.retention_secs
    }

    /// Insert samples restored from persistence.
    ///
    /// Samples are sorted by timestamp before insertion. The cache is then pruned
    /// using the newest restored timestamp.
    /// 
    /// # Arguments
    /// 
    /// * `samples` - the power samples to insert
    pub fn insert_many(&self, mut samples: Vec<PowerSample>) {
        if samples.is_empty() {
            return;
        }

        samples.sort_by_key(|sample| sample.ts);

        let newest_ts = samples.last().map(|sample| sample.ts).unwrap_or(0);

        let mut guard = self.inner.write().unwrap();
        guard.extend(samples);
        self.prune_locked(&mut guard, newest_ts);

        debug!("restored samples, cache size: {}", guard.len());
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
            .filter(|sample| sample.ts >= from_ts && sample.ts <= to_ts)
            .copied()
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

/// Average power represented during one time bucket.
///
/// Power values are averaged from all samples in the bucket.
/// Values are in their scaled real-world units.
#[derive(Debug, Copy, Clone)]
pub struct PowerAverage {
    /// Unix timestamp in seconds, normally the start of the bucket
    pub ts: UnixTs,
    /// Average power production during this bucket
    pub production: f64,
    /// Average power consumption during this bucket
    pub consumption: f64,
    /// Average battery state of charge in percent, 0-100
    pub batt_soc: f64,
}

/// Convert power samples into per-bucket average power values.
///
/// Samples are grouped by truncated bucket start. Each returned item contains
/// the average values for all samples in that bucket.
///
/// For example, with 5-minute buckets:
///
/// - a sample at `16:34:58` belongs to bucket `16:30:00`
/// - a sample at `16:39:58` belongs to bucket `16:35:00`
///
/// # Arguments
///
/// * `samples` - the power samples to process
/// * `bucket_size` - the duration of each time bucket, e.g. 5 minutes or 1 hour
pub fn power_average(samples: &[PowerSample], bucket_size: Duration) -> Vec<PowerAverage> {
    let bucket_secs = bucket_size.as_secs();

    if samples.is_empty() || bucket_secs == 0 {
        return Vec::new();
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by_key(|sample| sample.ts);

    let mut result = Vec::new();

    let mut current_bucket_start = align_to_bucket(sorted[0].ts, bucket_secs);
    let mut production_sum = 0.0;
    let mut consumption_sum = 0.0;
    let mut batt_soc_sum = 0.0;
    let mut sample_count: u64 = 0;

    for sample in sorted {
        let sample_bucket_start = align_to_bucket(sample.ts, bucket_secs);

        if sample_bucket_start != current_bucket_start {
            result.push(PowerAverage {
                ts: current_bucket_start,
                production: production_sum / sample_count as f64,
                consumption: consumption_sum / sample_count as f64,
                batt_soc: batt_soc_sum / sample_count as f64,
            });

            current_bucket_start = sample_bucket_start;
            production_sum = 0.0;
            consumption_sum = 0.0;
            batt_soc_sum = 0.0;
            sample_count = 0;
        }

        production_sum += sample.production;
        consumption_sum += sample.consumption;
        batt_soc_sum += sample.batt_soc;
        sample_count += 1;
    }

    if sample_count > 0 {
        result.push(PowerAverage {
            ts: current_bucket_start,
            production: production_sum / sample_count as f64,
            consumption: consumption_sum / sample_count as f64,
            batt_soc: batt_soc_sum / sample_count as f64,
        });
    }

    result
}

/// Align a timestamp to the start of a bucket
///
/// # Arguments
///
/// * `ts` - the Unix timestamp to align
/// * `bucket_secs` - the size of the bucket in seconds
fn align_to_bucket(ts: UnixTs, bucket_secs: u64) -> UnixTs {
    ts - ts % bucket_secs
}

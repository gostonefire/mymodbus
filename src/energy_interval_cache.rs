//! Cache for energy total samples and calculated energy intervals
//!
//! This module provides a cache for storing periodic energy total readings
//! and calculating energy consumption/production over 15-minute intervals.

use std::collections::VecDeque;
use std::sync::RwLock;

pub type UnixTs = u64;

const FIFTEEN_MINUTES_SECS: UnixTs = 15 * 60;
const MAX_INTERVALS_48H: usize = 48 * 4;
const MAX_SAMPLES: usize = MAX_INTERVALS_48H + 1;

/// A sample of energy total counters at a specific timestamp
#[derive(Debug, Copy, Clone)]
pub struct EnergyTotalSample {
    /// Unix timestamp in seconds
    pub ts: UnixTs,
    /// Total energy fed into the grid in kWh
    pub feed_in_energy_total: f64,
    /// Total energy consumed from the grid in kWh
    pub grid_consumption_energy_total: f64,
}

/// Calculated energy metrics for a specific time interval
#[derive(Debug, Copy, Clone)]
pub struct EnergyInterval {
    /// Start timestamp of the interval (exclusive)
    pub from_ts: UnixTs,

    /// End timestamp of the interval (inclusive)
    pub to_ts: UnixTs,

    /// kWh fed into the grid during this interval
    pub feed_in_energy: f64,

    /// kWh consumed from the grid during this interval
    pub grid_consumption_energy: f64,
}

/// Thread-safe cache for energy total samples
pub struct EnergyIntervalCache {
    samples: RwLock<VecDeque<EnergyTotalSample>>,
}

impl EnergyIntervalCache {
    /// Creates a new empty energy interval cache
    pub fn new() -> Self {
        Self {
            samples: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
        }
    }

    /// Clears all samples from the cache
    pub fn clear(&self) {
        let mut guard = self.samples.write().unwrap();
        guard.clear();
    }

    /// Inserts a new energy total sample into the cache
    ///
    /// If the new sample is not monotonic or does not follow the expected
    /// 15-minute interval, the cache is cleared before insertion.
    ///
    /// # Arguments
    ///
    /// * `sample` - the energy total sample to insert
    pub fn insert(&self, sample: EnergyTotalSample) {
        let mut guard = self.samples.write().unwrap();

        if let Some(last) = guard.back() {
            let has_expected_timestamp =
                sample.ts.saturating_sub(last.ts) == FIFTEEN_MINUTES_SECS;

            let counters_are_monotonic =
                sample.feed_in_energy_total >= last.feed_in_energy_total
                    && sample.grid_consumption_energy_total >= last.grid_consumption_energy_total;

            if sample.ts <= last.ts || !has_expected_timestamp || !counters_are_monotonic {
                guard.clear();
            }
        }

        guard.push_back(sample);

        while guard.len() > MAX_SAMPLES {
            guard.pop_front();
        }
    }

    /// Returns the latest stored energy total sample, if any
    pub fn latest(&self) -> Option<EnergyTotalSample> {
        let guard = self.samples.read().unwrap();
        guard.back().copied()
    }

    /// Queries calculated 15-minute energy intervals within a time range
    ///
    /// The query uses the nearest stored end timestamp <= `to_ts`,
    /// then walks backwards until `from_ts`.
    ///
    /// # Arguments
    ///
    /// * `from_ts` - start of the time range (Unix timestamp)
    /// * `to_ts` - end of the time range (Unix timestamp)
    pub fn query_intervals(&self, from_ts: UnixTs, to_ts: UnixTs) -> Vec<EnergyInterval> {
        let guard = self.samples.read().unwrap();

        if guard.len() < 2 || from_ts > to_ts {
            return Vec::new();
        }

        let samples: Vec<EnergyTotalSample> = guard.iter().copied().collect();

        let mut result = Vec::new();

        let Some(mut idx) = samples.iter().rposition(|sample| sample.ts <= to_ts) else {
            return result;
        };

        while idx > 0 {
            let current = samples[idx];
            let previous = samples[idx - 1];

            if current.ts < from_ts {
                break;
            }

            let interval_is_expected_size =
                current.ts.saturating_sub(previous.ts) == FIFTEEN_MINUTES_SECS;

            let feed_in_delta =
                current.feed_in_energy_total - previous.feed_in_energy_total;

            let grid_consumption_delta =
                current.grid_consumption_energy_total - previous.grid_consumption_energy_total;

            let counters_are_monotonic =
                feed_in_delta >= 0.0 && grid_consumption_delta >= 0.0;

            if interval_is_expected_size && counters_are_monotonic {
                result.push(EnergyInterval {
                    from_ts: previous.ts,
                    to_ts: current.ts,
                    feed_in_energy: feed_in_delta,
                    grid_consumption_energy: grid_consumption_delta,
                });
            }

            idx -= 1;
        }

        result.reverse();
        result
    }
}

impl Default for EnergyIntervalCache {
    fn default() -> Self {
        Self::new()
    }
}

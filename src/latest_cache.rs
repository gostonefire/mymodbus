//! Cache for the most recent values of Modbus registers
//!
//! This module provides an in-memory cache for the latest read values of specific registers,
//! allowing for efficient retrieval without needing to access historical data or poll the hardware again.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::history_cache::UnixTs;

/// Latest cached value for a frequently requested register.
#[derive(Debug, Copy, Clone)]
pub struct LatestValue {
    /// Unix timestamp in seconds when the value was read
    pub ts: UnixTs,
    /// Scaled real-world value
    pub value: f64,
}

/// In-memory cache for latest-only values.
///
/// This is intended for values that are polled regularly and frequently requested,
/// but where keeping a history is unnecessary.
pub struct LatestCache {
    inner: RwLock<HashMap<String, LatestValue>>,
}

impl LatestCache {
    /// Create an empty latest-value cache.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or replace the latest value for a register id.
    ///
    /// # Arguments
    ///
    /// * `id` - register unique identifier
    /// * `value` - the scaled real-world value
    /// * `ts` - Unix timestamp in seconds when the value was read
    pub fn insert(&self, id: impl Into<String>, value: f64, ts: UnixTs) {
        let mut guard = self.inner.write().unwrap();
        guard.insert(id.into(), LatestValue { ts, value });
    }

    /// Return the latest value for a register id.
    ///
    /// # Arguments
    ///
    /// * `id` - register unique identifier
    pub fn get(&self, id: &str) -> Option<LatestValue> {
        let guard = self.inner.read().unwrap();
        guard.get(id).copied()
    }
}

impl Default for LatestCache {
    fn default() -> Self {
        Self::new()
    }
}

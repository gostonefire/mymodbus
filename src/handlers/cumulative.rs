//! Handler for hcumulative data queries
//!
//! Provides an endpoint to query cumulative power data from a specified time.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};
use crate::handlers::{handle_history_cumulative_json, json_response};
use crate::history_cache::HistoryCache;

/// Handles requests for cumulative data
///
/// # Arguments
///
/// * `path` - the request path including query parameters (e.g., "/history?from_ts=...&to_ts=...")
/// * `history_cache` - shared history cache to query
pub fn handle_cumulative(path: &str, history_cache: Arc<HistoryCache>) -> Result<String> {
    let query = path.split_once('?').map(|(_, query)| query).unwrap_or("");
    let mut from_ts: Option<u64> = None;
    let mut to_ts: Option<u64> = None;

    for part in query.split('&').filter(|s| !s.is_empty()) {
        if let Some(value) = part.strip_prefix("from_ts=") {
            from_ts = value.parse::<u64>().ok();
        } else if let Some(value) = part.strip_prefix("to_ts=") {
            to_ts = value.parse::<u64>().ok();
        }
    }
    
    let to_ts = to_ts.unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());

    match from_ts {
        Some(from_ts) => {
            handle_history_cumulative_json(history_cache.clone(), from_ts, to_ts)
                .map(|json| json_response("200 OK", json))
        }
        _ => Err(anyhow!("invalid request: /cumulative requires from_ts query parameters")),
    }
}

//! Handler for historical data queries
//!
//! Provides an endpoint to query historical power data within a specified time range.

use std::sync::Arc;
use anyhow::{anyhow, Result};
use crate::handlers::{handle_history_query_json, json_response};
use crate::history_cache::HistoryCache;

/// Handles requests for historical data
///
/// # Arguments
///
/// * `path` - the request path including query parameters (e.g., "/history?from_ts=...&to_ts=...")
/// * `history_cache` - shared history cache to query
pub fn handle_history(path: &str, history_cache: Arc<HistoryCache>) -> Result<String> {
    let query = path.split_once('?').map(|(_, query)| query).unwrap_or("");
    let mut from_ts: Option<u64> = None;
    let mut to_ts: Option<u64> = None;
    let mut interval: Option<u64> = None;

    for part in query.split('&').filter(|s| !s.is_empty()) {
        if let Some(value) = part.strip_prefix("from_ts=") {
            from_ts = value.parse::<u64>().ok();
        } else if let Some(value) = part.strip_prefix("to_ts=") {
            to_ts = value.parse::<u64>().ok();
        } else if let Some(value) = part.strip_prefix("interval=") {
            interval = value.parse::<u64>().ok();
        }
    }

    match (from_ts, to_ts) {
        (Some(from_ts), Some(to_ts)) => {
            handle_history_query_json(history_cache.clone(), from_ts, to_ts, interval)
                .map(|json| json_response("200 OK", json))
        }
        _ => Err(anyhow!("invalid request: /history requires from_ts and to_ts query parameters")),
    }
}

//! HTTP handler for energy intervals
//!
//! This module provides the handler for querying 15-minute energy intervals.

use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::energy_interval_cache::EnergyIntervalCache;
use crate::handlers::json_response;

/// Handles HTTP requests for energy intervals
///
/// Parses `from_ts` and `to_ts` from the query string and returns
/// calculated energy intervals as a JSON response.
///
/// # Arguments
///
/// * `path` - the full request path including query string
/// * `energy_interval_cache` - shared cache to query intervals from
pub fn handle_energy_intervals(
    path: &str,
    energy_interval_cache: Arc<EnergyIntervalCache>,
) -> Result<String> {
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

    let Some(from_ts) = from_ts else {
        return Err(anyhow!("missing from_ts"));
    };

    let Some(to_ts) = to_ts else {
        return Err(anyhow!("missing to_ts"));
    };

    if from_ts > to_ts {
        return Err(anyhow!("invalid range: from_ts must be <= to_ts"));
    }

    let intervals = energy_interval_cache.query_intervals(from_ts, to_ts);

    let mut body = String::new();

    body.push('{');
    body.push_str(&format!("\"from_ts\":{},", from_ts));
    body.push_str(&format!("\"to_ts\":{},", to_ts));
    body.push_str("\"intervals\":[");

    for (idx, interval) in intervals.iter().enumerate() {
        if idx > 0 {
            body.push(',');
        }

        body.push_str(&format!(
            "{{\"from_ts\":{},\"to_ts\":{},\"feed_in_energy\":{},\"grid_consumption_energy\":{}}}",
            interval.from_ts,
            interval.to_ts,
            interval.feed_in_energy,
            interval.grid_consumption_energy,
        ));
    }

    body.push_str("]}");

    Ok(json_response("200 OK", body))
}

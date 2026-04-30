//! HTTP request handlers for the Mymodbus application
//!
//! This module contains various handlers for HTTP endpoints, providing access to
//! Modbus register values and historical data.

use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use crate::history_cache::{power_deltas, HistoryCache, PowerDelta};
use crate::manager_modbus::RegisterValue;

pub mod id;
pub mod address;
pub mod history;
pub mod favicon;
pub mod empty;
pub mod bad_request;
pub mod cumulative;

pub use id::handle_id;
pub use address::handle_address;
pub use history::handle_history;
pub use favicon::handle_favicon;
pub use empty::handle_empty;
pub use bad_request::handle_bad_request;

/// Formats a JSON response
///
/// # Arguments
///
/// * `status` - the HTTP status line (e.g., "200 OK")
/// * `body` - the JSON body of the response
fn json_response(status: &str, body: String) -> String {
    format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    )
}

/// Formats an empty HTTP response
///
/// # Arguments
///
/// * `status` - the HTTP status line (e.g., "204 No Content")
fn empty_response(status: &str) -> String {
    format!(
        "HTTP/1.1 {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        status
    )
}

/// Helper function to format a Modbus register value as an HTTP response
///
/// # Arguments
///
/// * `data` - the result of a Modbus register read
fn http_response(data: anyhow::Result<RegisterValue>) -> String {
    let value = match data {
        Ok(data) => match data {
            RegisterValue::String(value) => format!("\"{}\"", value),
            _ => data
                .to_f64()
                .map(|v| v.to_string())
                .unwrap_or_else(|e| format!("\"{}\"", e)),
        },
        Err(e) => format!("\"{}\"", e),
    };

    json_response("200 OK", format!("{{\"data\": {}}}", value))
}

/// Query the in-memory history cache and return a JSON string
///
/// # Arguments
///
/// * `history_cache` - shared history cache to query
/// * `from_ts` - start timestamp for the query
/// * `to_ts` - end timestamp for the query
pub fn handle_history_cumulative_json(
    history_cache: Arc<HistoryCache>,
    from_ts: u64,
    to_ts: u64,
) -> anyhow::Result<String> {
    if from_ts > to_ts {
        return Err(anyhow!("invalid range: from_ts must be <= to_ts"));
    }

    let sample = history_cache.cumulative(from_ts, to_ts)
        .ok_or(anyhow!("no sample found for range"))?;
    
    Ok(cumulative_response_json(from_ts, to_ts, sample))
}

/// Helper function to format cumulative data as a JSON string
///
/// # Arguments
///
/// * `from_ts` - start timestamp of the data
/// * `to_ts` - end timestamp of the data
/// * `sample` - the cumulative power sample
fn cumulative_response_json(
    from_ts: u64,
    to_ts: u64,
    sample: PowerDelta,
) -> String {
    let mut out = String::new();

    out.push('{');
    out.push_str(&format!("\"from_ts\":{},", from_ts));
    out.push_str(&format!("\"to_ts\":{},", to_ts));
    out.push_str("\"sample\":");

    out.push_str(&format!(
        "{{\"ts\":{},\"produced\":{},\"consumed\":{},\"exported\":{}}}",
        sample.ts, sample.produced, sample.consumed, sample.exported
    ));

    out.push_str("}");
    out
}

/// Query the in-memory history cache and return a JSON string
///
/// # Arguments
///
/// * `history_cache` - shared history cache to query
/// * `from_ts` - start timestamp for the query
/// * `to_ts` - end timestamp for the query
/// * `interval` - interval between samples in minutes (i.e., bucket size)
pub fn handle_history_query_json(
    history_cache: Arc<HistoryCache>,
    from_ts: u64,
    to_ts: u64,
    interval: Option<u64>,
) -> anyhow::Result<String> {
    if from_ts > to_ts {
        return Err(anyhow!("invalid range: from_ts must be <= to_ts"));
    }

    let samples = history_cache.query(from_ts, to_ts);
    let values = power_deltas(&samples, Duration::from_secs(interval.unwrap_or(5) * 60));
    Ok(history_response_json(from_ts, to_ts, false, &values))
}

/// Helper function to format historical data as a JSON string
///
/// # Arguments
///
/// * `from_ts` - start timestamp of the data
/// * `to_ts` - end timestamp of the data
/// * `truncated` - whether the data was truncated
/// * `samples` - the historical power samples
fn history_response_json(
    from_ts: u64,
    to_ts: u64,
    truncated: bool,
    samples: &[PowerDelta],
) -> String {
    let mut out = String::new();

    out.push('{');
    out.push_str(&format!("\"from_ts\":{},", from_ts));
    out.push_str(&format!("\"to_ts\":{},", to_ts));
    out.push_str(&format!("\"truncated\":{},", truncated));
    out.push_str("\"samples\":[");

    for (idx, sample) in samples.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"ts\":{},\"produced\":{},\"consumed\":{},\"exported\":{}}}",
            sample.ts, sample.produced, sample.consumed, sample.exported
        ));
    }

    out.push_str("]}");
    out
}

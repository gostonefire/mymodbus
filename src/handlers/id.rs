//! Handler for register queries by unique ID
//!
//! Provides an endpoint to query a Modbus register using its predefined unique identifier.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use anyhow::Result;
use log::debug;
use crate::handlers::http_response;
use crate::latest_cache::LatestCache;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest, RegisterValue};

/// Handles requests to query a register by its unique ID
///
/// # Arguments
///
/// * `path` - the request path (e.g., "/id/some_id")
/// * `tx_request` - channel to send Modbus requests
/// * `latest_cache` - cache for latest-only values populated by the poller
pub fn handle_id(path: &str, tx_request: &Sender<ModbusRequest>, latest_cache: &Arc<LatestCache>) -> Result<String> {
    let value = path.trim_start_matches("/id/").trim_end_matches('/');

    if let Some(cached) = latest_cache.get(value) {
        debug!("Returning cached value for register {}", value);
        return Ok(http_response(Ok(RegisterValue::F64(cached.value))));
    }

    Ok(http_response(send_request(
        &tx_request,
        RegisterRequest::UniqueId(value.to_string()),
    )))
}

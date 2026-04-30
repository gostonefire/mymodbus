//! Handler for register queries by unique ID
//!
//! Provides an endpoint to query a Modbus register using its predefined unique identifier.

use std::sync::mpsc::Sender;
use anyhow::Result;
use crate::handlers::http_response;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};

/// Handles requests to query a register by its unique ID
///
/// # Arguments
///
/// * `path` - the request path (e.g., "/id/some_id")
/// * `tx_request` - channel to send Modbus requests
pub fn handle_id(path: &str, tx_request: &Sender<ModbusRequest>) -> Result<String> {
    let value = path.trim_start_matches("/id/").trim_end_matches('/');
    Ok(http_response(send_request(
        &tx_request,
        RegisterRequest::UniqueId(value.to_string()),
    )))
}

//! Handler for register queries by address
//!
//! Provides an endpoint to query a Modbus register by its raw address.

use std::sync::mpsc::Sender;
use anyhow::Result;
use crate::handlers::http_response;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};

/// Handles requests to query a register by its address
///
/// # Arguments
///
/// * `path` - the request path (e.g., "/address/123")
/// * `tx_request` - channel to send Modbus requests
pub fn handle_address(path: &str, tx_request: &Sender<ModbusRequest>) -> Result<String> {
    let value = path.trim_start_matches("/address/").trim_end_matches('/');
    Ok(http_response(send_request(
        &tx_request,
        RegisterRequest::Raw(value.to_string()),
    )))
}

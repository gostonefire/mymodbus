//! Handler for empty path requests
//!
//! Provides a simple status check for the root endpoint.

use anyhow::Result;
use crate::handlers::json_response;

/// Handles requests to the root path
pub fn handle_empty() -> Result<String> {
    Ok(json_response("200 OK", "{\"status\":\"ok\"}".to_string()))
}

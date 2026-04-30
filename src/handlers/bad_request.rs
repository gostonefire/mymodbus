//! Handler for bad requests
//!
//! Provides a standardized way to return 400 Bad Request responses.

use anyhow::Error;
use crate::handlers::json_response;

/// Handles bad requests by returning a 400 Bad Request JSON response
///
/// # Arguments
///
/// * `error` - the error that occurred
pub fn handle_bad_request(error: Error) -> String {
    json_response("400 Bad Request", format!("{{\"error\":\"{}\"}}", error))
}
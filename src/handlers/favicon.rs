//! Handler for favicon requests
//!
//! Provides an empty response for favicon.ico requests.

use anyhow::Result;
use crate::handlers::empty_response;

/// Handles requests for favicon.ico
pub fn handle_favicon() -> Result<String> {
    Ok(empty_response("204 No Content"))
}
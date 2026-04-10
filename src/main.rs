//! # Modbus CLI Tool
//!
//! This application provides a command-line interface for reading data from a Modbus device
//! (e.g., an inverter) via a serial port using the RTU protocol.
//!
//! The tool resolves register identifiers into their corresponding Modbus addresses
//! and data types using a predefined register database, performs the serial communication,
//! and displays the results.

mod manager_modbus;
mod registers;
mod initialization;
mod logging;
mod persistence;
mod http_server;

use crate::manager_modbus::{run, RegisterRequest, RegisterValue};
use anyhow::Result;
use std::thread;
use log::error;
use crate::http_server::run_server;
use crate::initialization::config;

/// Main entry point for the Modbus HTTP server.
///
fn main() -> Result<()> {
    let config = config()?;

    let (tx_result, rx_result) = std::sync::mpsc::channel::<Result<RegisterValue>>();
    let (tx_request, rx_request) = std::sync::mpsc::channel::<RegisterRequest>();

    thread::spawn(move || {
        if let Err(r) = run(config.modbus.serial_port, tx_result, rx_request) {
            error!("modbus error: {}", r);
        }
    });

    run_server(
        config.web_server.bind_address.parse()?,
        config.web_server.bind_port,
        tx_request,
        rx_result,
    )?;

    Ok(())
}

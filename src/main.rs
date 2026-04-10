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

use crate::manager_modbus::{run, RegisterRequest, RegisterValue};
use anyhow::{anyhow, Result};
use std::thread;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use log::error;
use crate::initialization::config;

const HTTP_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";

/// Main entry point for the Modbus CLI tool.
///
/// This function:
/// 1. Parses the command-line arguments to get the register identifier.
/// 2. Initializes the Modbus client.
/// 3. Resolves the register metadata.
/// 4. Reads the register value and displays it.
///
/// # Arguments
///
/// * `args` - Command-line arguments from the environment.
fn main() -> Result<()> {
    let config = config()?;

    let (tx_result, rx_result) = std::sync::mpsc::channel::<Result<RegisterValue>>();
    let (tx_request, rx_request) = std::sync::mpsc::channel::<RegisterRequest>();

    thread::spawn(move || {
        if let Err(r) = run(config.modbus.serial_port, tx_result, rx_request) {
            log::error!("modbus error: {}", r);
        }
    });

    let socket_addr = SocketAddr::new(config.web_server.bind_address.parse()?, config.web_server.bind_port);
    let listener = TcpListener::bind(socket_addr)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(_) => {
                        let request = String::from_utf8_lossy(&buffer[..]);

                        let request_line = request.lines().next().unwrap_or("");
                        let path = request_line
                            .strip_prefix("GET ")
                            .and_then(|rest| rest.split_whitespace().next());

                        let result = match path {
                            Some(path) if path.starts_with("/id/") => {
                                let value = path.trim_start_matches("/id/").trim_end_matches('/');
                                tx_request.send(RegisterRequest::UniqueId(value.to_string()))?;
                                rx_result.recv()?
                            }
                            Some(path) if path.starts_with("/address/") => {
                                let value = path.trim_start_matches("/address/").trim_end_matches('/');
                                tx_request.send(RegisterRequest::Raw(value.to_string()))?;
                                rx_result.recv()?
                            }
                            _ => {
                                Err(anyhow!("unsupported request"))
                            }
                        };

                        if let Err(e) = stream.write(http_response(result).as_bytes()) {
                            error!("could not write to stream: {}", e);
                        }
                    },
                    Err(e) => { error!("failed to read from stream: {}", e); }
                }
            },
            Err(e) => { error!("failed to get stream for requestor: {}", e); }
        }
    }

    Ok(())
}

/// Creates an HTTP response string with data in json
///
/// # Arguments
///
/// * 'data' - data to include in response
fn http_response(data: Result<RegisterValue>) -> String {

    let value = match data {
        Ok(data) => {
            match data {
                RegisterValue::String(value) => value,
                _ => {
                    data.to_f64().map(|v| v.to_string()).unwrap_or_else(|e| e.to_string())
                }
            }
        },
        Err(e) => {
            e.to_string()
        }
    };

    format!("{}{{\"data\": {}}}", HTTP_RESPONSE, value)
}

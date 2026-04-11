mod manager_modbus;
mod registers;
mod initialization;
mod logging;
mod persistence;
mod http_server;
mod shutdown;

use crate::http_server::run_server;
use crate::initialization::config;
use crate::manager_modbus::{run, RegisterRequest, RegisterValue};
use crate::shutdown::spawn_shutdown_listener;
use anyhow::Result;
use log::error;
use std::sync::mpsc;
use std::thread;

fn main() -> Result<()> {
    let config = config()?;

    let (tx_result, rx_result) = mpsc::channel::<Result<RegisterValue>>();
    let (tx_request, rx_request) = mpsc::channel::<RegisterRequest>();
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<()>();

    let modbus_handle = thread::spawn(move || {
        if let Err(r) = run(config.modbus.serial_port, tx_result, rx_request) {
            error!("modbus error: {}", r);
        }
    });

    let shutdown_handle = spawn_shutdown_listener(tx_shutdown)?;

    let server_result = run_server(
        config.web_server.bind_address.parse()?,
        config.web_server.bind_port,
        tx_request.clone(),
        rx_result,
        rx_shutdown,
    );

    let _ = tx_request.send(RegisterRequest::Exit);
    let _ = modbus_handle.join();
    let _ = shutdown_handle.join();

    server_result
}

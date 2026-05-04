//! HTTP server for the Mymodbus application
//!
//! Provides an API to query Modbus registers and historical data.

use anyhow::{anyhow, Result};
use log::{error, debug, info};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use crate::handlers::{
    handle_address, handle_bad_request, handle_empty, handle_favicon, handle_history, handle_id,
};
use crate::history_cache::HistoryCache;
use crate::latest_cache::LatestCache;
use crate::energy_interval_cache::EnergyIntervalCache;
use crate::handlers::energy_intervals::handle_energy_intervals;
use crate::manager_modbus::ModbusRequest;


/// Runs the HTTP server
///
/// # Arguments
///
/// * `bind_address` - IP address to bind the server to
/// * `bind_port` - port to bind the server to
/// * `tx_request` - channel to send Modbus requests
/// * `rx_shutdown` - channel to receive shutdown signal
/// * `history_cache` - shared history cache for historical data queries
/// * `latest_cache` - cache for latest-only values populated by the poller
/// * `energy_interval_cache` - cache for energy interval values populated by the poller
pub fn run_server(
    bind_address: IpAddr,
    bind_port: u16,
    tx_request: Sender<ModbusRequest>,
    rx_shutdown: Receiver<()>,
    history_cache: Arc<HistoryCache>,
    latest_cache: Arc<LatestCache>,
    energy_interval_cache: Arc<EnergyIntervalCache>,
) -> Result<()> {
    let socket_addr = SocketAddr::new(bind_address, bind_port);

    info!("starting http server on {}", socket_addr);

    let listener = TcpListener::bind(socket_addr)
        .map_err(|e| {
            error!("failed to bind http server to {}: {}", socket_addr, e);
            e
        })?;

    listener.set_nonblocking(true)
        .map_err(|e| {
            error!("failed to set http server listener to nonblocking mode: {}", e);
            e
        })?;

    info!("http server listening on {}", socket_addr);

    loop {
        if rx_shutdown.try_recv().is_ok() {
            info!("shutdown requested, stopping http server");
            break;
        }

        match listener.accept() {
            Ok((mut stream, addr)) => {
                debug!("accepted http connection from {}", addr);

                if let Err(e) = stream.set_nonblocking(false) {
                    error!("failed to set http client stream to blocking mode: {}", e);
                    continue;
                }

                if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(5))) {
                    error!("failed to set http client stream read timeout: {}", e);
                    continue;
                }
                let mut buffer = [0; 1024];

                match stream.read(&mut buffer) {
                    Ok(0) => {
                        debug!("client {} disconnected before sending a request", addr);
                    }
                    Ok(bytes_read) => {
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                        let request_line = request.lines().next().unwrap_or("");
                        debug!("request from {}: {}", addr, request_line);

                        let path = request_line
                            .strip_prefix("GET ")
                            .and_then(|rest| rest.split_whitespace().next());

                        let response = match path {
                            Some("/") => {
                                handle_empty()
                            }
                            Some("/favicon.ico") => {
                                handle_favicon()
                            }
                            Some(path) if path.starts_with("/id/") => {
                                handle_id(path, &tx_request, &latest_cache)
                            }
                            Some(path) if path.starts_with("/address/") => {
                                handle_address(path, &tx_request)
                            }
                            Some(path) if path.starts_with("/history") => {
                                handle_history(path, history_cache.clone())
                            }
                            Some(path) if path.starts_with("/energy-intervals") => {
                                handle_energy_intervals(path, energy_interval_cache.clone())
                            }
                            _ => Err(anyhow!("unsupported request")),
                        };

                        let body = response.unwrap_or_else(|e| {
                            handle_bad_request(e)
                        });

                        if let Err(e) = stream.write_all(body.as_bytes()) {
                            error!("could not write to stream: {}", e);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        debug!("client {} had no data available before timeout", addr);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        debug!("client {} timed out before sending a request", addr);
                    }
                    Err(e) => error!("failed to read from client {} stream: {}", addr, e),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => error!("failed to accept requestor: {}", e),
        }
    }

    info!("http server stopped");

    Ok(())
}

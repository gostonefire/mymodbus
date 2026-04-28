//! HTTP server for the Mymodbus application
//!
//! Provides an API to query Modbus registers and historical data.

use anyhow::{anyhow, Result};
use log::error;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use crate::history_cache::HistoryCache;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest, RegisterValue};
use crate::poller::PowerSample;

const HTTP_RESPONSE: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\n\r\n";

/// Runs the HTTP server
///
/// # Arguments
///
/// * `bind_address` - IP address to bind the server to
/// * `bind_port` - port to bind the server to
/// * `tx_request` - channel to send Modbus requests
/// * `rx_shutdown` - channel to receive shutdown signal
/// * `history_cache` - shared history cache for historical data queries
pub fn run_server(
    bind_address: IpAddr,
    bind_port: u16,
    tx_request: Sender<ModbusRequest>,
    rx_shutdown: Receiver<()>,
    history_cache: Arc<HistoryCache>,
) -> Result<()> {
    let socket_addr = SocketAddr::new(bind_address, bind_port);

    log::info!("starting http server on {}", socket_addr);

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

    log::info!("http server listening on {}", socket_addr);

    loop {
        if rx_shutdown.try_recv().is_ok() {
            log::info!("shutdown requested, stopping http server");
            break;
        }

        match listener.accept() {
            Ok((mut stream, _addr)) => {
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
                        error!("client disconnected before sending a request");
                    }
                    Ok(bytes_read) => {
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                        let request_line = request.lines().next().unwrap_or("");
                        let path = request_line
                            .strip_prefix("GET ")
                            .and_then(|rest| rest.split_whitespace().next());

                        let response = match path {
                            Some(path) if path.starts_with("/id/") => {
                                let value = path.trim_start_matches("/id/").trim_end_matches('/');
                                Ok(http_response(send_request(
                                    &tx_request,
                                    RegisterRequest::UniqueId(value.to_string()),
                                )))
                            }
                            Some(path) if path.starts_with("/address/") => {
                                let value = path.trim_start_matches("/address/").trim_end_matches('/');
                                Ok(http_response(send_request(
                                    &tx_request,
                                    RegisterRequest::Raw(value.to_string()),
                                )))
                            }
                            Some(path) if path.starts_with("/history") => {
                                let query = path.split_once('?').map(|(_, query)| query).unwrap_or("");
                                let mut from_ts: Option<u64> = None;
                                let mut to_ts: Option<u64> = None;

                                for part in query.split('&').filter(|s| !s.is_empty()) {
                                    if let Some(value) = part.strip_prefix("from_ts=") {
                                        from_ts = value.parse::<u64>().ok();
                                    } else if let Some(value) = part.strip_prefix("to_ts=") {
                                        to_ts = value.parse::<u64>().ok();
                                    }
                                }

                                match (from_ts, to_ts) {
                                    (Some(from_ts), Some(to_ts)) => {
                                        handle_history_query_json(history_cache.clone(), from_ts, to_ts)
                                            .map(|json| format!("{}{}", HTTP_RESPONSE, json))
                                    }
                                    _ => Err(anyhow!(
                                            "invalid request: /history requires from_ts and to_ts query parameters"
                                        )),
                                }
                            }
                            _ => Err(anyhow!("unsupported request")),
                        };

                        let body = response.unwrap_or_else(|e| {
                            format!("{}{{\"error\":\"{}\"}}", HTTP_RESPONSE, e)
                        });

                        if let Err(e) = stream.write(body.as_bytes()) {
                            error!("could not write to stream: {}", e);
                        }
                    }
                    Err(e) => error!("failed to read from stream: {}", e),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => error!("failed to accept requestor: {}", e),
        }
    }

    log::info!("http server stopped");

    Ok(())
}

/// Helper function to format a Modbus register value as an HTTP response
///
/// # Arguments
///
/// * `data` - the result of a Modbus register read
fn http_response(data: Result<RegisterValue>) -> String {
    let value = match data {
        Ok(data) => match data {
            RegisterValue::String(value) => value,
            _ => data
                .to_f64()
                .map(|v| v.to_string())
                .unwrap_or_else(|e| e.to_string()),
        },
        Err(e) => e.to_string(),
    };

    format!("{}{{\"data\": {}}}", HTTP_RESPONSE, value)
}

/// Query the in-memory history cache and return a JSON string
///
/// # Arguments
///
/// * `history_cache` - shared history cache to query
/// * `from_ts` - start timestamp for the query
/// * `to_ts` - end timestamp for the query
pub fn handle_history_query_json(
    history_cache: Arc<HistoryCache>,
    from_ts: u64,
    to_ts: u64,
) -> Result<String> {
    if from_ts > to_ts {
        return Err(anyhow!("invalid range: from_ts must be <= to_ts"));
    }

    let samples = history_cache.query(from_ts, to_ts);
    Ok(history_response_json(from_ts, to_ts, false, &samples))
}

/// Helper function to format historical data as a JSON string
///
/// # Arguments
///
/// * `from_ts` - start timestamp of the data
/// * `to_ts` - end timestamp of the data
/// * `truncated` - whether the data was truncated
/// * `samples` - the historical power samples
fn history_response_json(
    from_ts: u64,
    to_ts: u64,
    truncated: bool,
    samples: &[PowerSample],
) -> String {
    let mut out = String::new();

    out.push('{');
    out.push_str(&format!("\"from_ts\":{},", from_ts));
    out.push_str(&format!("\"to_ts\":{},", to_ts));
    out.push_str(&format!("\"truncated\":{},", truncated));
    out.push_str("\"samples\":[");

    for (idx, sample) in samples.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"ts\":{},\"produced\":{},\"consumed\":{},\"exported\":{}}}",
            sample.ts, sample.produced, sample.consumed, sample.exported
        ));
    }

    out.push_str("]}");
    out
}
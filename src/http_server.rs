use anyhow::{anyhow, Result};
use log::error;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::mpsc::{Receiver, Sender};

use crate::manager_modbus::{RegisterRequest, RegisterValue};

const HTTP_RESPONSE: &str = "HTTP/1.1 200 OK\r\n\r\n";

pub fn run_server(
    bind_address: IpAddr,
    bind_port: u16,
    tx_request: Sender<RegisterRequest>,
    rx_result: Receiver<Result<RegisterValue>>,
) -> Result<()> {
    let socket_addr = SocketAddr::new(bind_address, bind_port);
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
                            _ => Err(anyhow!("unsupported request")),
                        };

                        if let Err(e) = stream.write(http_response(result).as_bytes()) {
                            error!("could not write to stream: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("failed to read from stream: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("failed to get stream for requestor: {}", e);
            }
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
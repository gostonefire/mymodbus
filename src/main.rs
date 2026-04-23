mod manager_modbus;
mod registers;
mod initialization;
mod logging;
mod persistence;
mod http_server;
mod shutdown;
mod poller;
mod history_cache;

use crate::http_server::run_server;
use crate::history_cache::HistoryCache;
use crate::initialization::config;
use crate::manager_modbus::{run, send_exit, ModbusRequest};
use crate::poller::spawn_poller;
use crate::persistence::HistoryStore;
use crate::shutdown::spawn_shutdown_listener;
use anyhow::Result;
use log::{error, warn};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

fn main() -> Result<()> {
    let config = config()?;

    let (tx_request, rx_request) = mpsc::channel::<ModbusRequest>();
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<()>();

    let modbus_handle = thread::spawn(move || {
        if let Err(r) = run(config.modbus.serial_port, rx_request) {
            error!("modbus error: {}", r);
        }
    });

    let shutdown_handle = spawn_shutdown_listener(tx_shutdown)?;

    let history_store = HistoryStore::new(
        "data/history.snapshot",
        "data/history.journal",
        10_000,
    );

    let cached_samples = match history_store.load() {
        Ok(buffer) => {
            warn!("loaded {} historical samples from disk", buffer.len());
            buffer
        }
        Err(err) => {
            warn!("failed to load history from disk: {}", err);
            Vec::new()
        }
    };

    let history_cache = Arc::new(HistoryCache::from_samples(
        48 * 60 * 60,
        cached_samples,
    ));

    let poller_handle = spawn_poller(
        tx_request.clone(),
        history_store,
        history_cache.clone(),
        "produced_power".to_string(),
        "consumed_power".to_string(),
    );

    let server_result = run_server(
        config.web_server.bind_address.parse()?,
        config.web_server.bind_port,
        tx_request.clone(),
        rx_shutdown,
        history_cache.clone(),
    );

    let _ = send_exit(&tx_request);
    let _ = modbus_handle.join();
    let _ = poller_handle.join();
    let _ = shutdown_handle.join();

    server_result
}
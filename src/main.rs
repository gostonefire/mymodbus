//! Entry point for the Mymodbus application
//!
//! This module initializes the system, spawns necessary threads, and starts the HTTP server.

mod manager_modbus;
mod registers;
mod initialization;
mod logging;
mod http_server;
mod shutdown;
mod poller;
mod history_cache;
mod persistence;
pub mod handlers;

use crate::http_server::run_server;
use crate::history_cache::HistoryCache;
use crate::initialization::config;
use crate::manager_modbus::{run, send_exit, ModbusRequest};
use crate::poller::spawn_poller;
use crate::shutdown::spawn_shutdown_listener;
use anyhow::Result;
use log::{error, warn};
use std::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::persistence::Persistence;

/// Main entry point of the application
///
fn main() -> Result<()> {
    let config = config()?;

    let (tx_request, rx_request) = mpsc::channel::<ModbusRequest>();

    let (tx_os_shutdown, rx_os_shutdown) = mpsc::channel::<()>();
    let (tx_server_shutdown, rx_server_shutdown) = mpsc::channel::<()>();
    let (tx_poller_shutdown, rx_poller_shutdown) = mpsc::channel::<()>();

    let shutdown_handle = spawn_shutdown_listener(tx_os_shutdown)?;

    let shutdown_dispatcher_handle = thread::spawn(move || {
        if rx_os_shutdown.recv().is_ok() {
            let _ = tx_server_shutdown.send(());
            let _ = tx_poller_shutdown.send(());
        }
    });

    let modbus_handle = thread::spawn(move || {
        if let Err(r) = run(config.modbus.serial_port, rx_request, config.modbus.mock) {
            error!("modbus error: {}", r);
        }
    });

    let history_cache = Arc::new(HistoryCache::new(
        config.cache.in_memory_hours * 60 * 60,
    ));

    let persistence = Arc::new(Mutex::new(Persistence::new(
        config.cache.persistence_path.parse()?,
        config.cache.persistence_rotate_samples,
        Duration::from_secs(config.cache.persistence_days * 24 * 60 * 60),
    )?));

    {
        let persistence_guard = persistence.lock().unwrap();

        if let Err(err) = persistence_guard.cleanup_old_files() {
            warn!("persistence cleanup failed: {err}");
        }

        let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cutoff_ts = now_ts.saturating_sub(history_cache.retention_secs());

        match persistence_guard.load_since(cutoff_ts) {
            Ok(samples) => history_cache.insert_many(samples),
            Err(err) => warn!("failed to restore persisted history cache: {err}"),
        }
    }

    let poller_handle = spawn_poller(
        tx_request.clone(),
        rx_poller_shutdown,
        history_cache.clone(),
        persistence.clone(),
        "pv_energy_total".to_string(),
        "load_energy_total".to_string(),
        "feed_in_energy_total".to_string(),
        "battery_soc".to_string(),
    );

    let server_result = run_server(
        config.web_server.bind_address.parse()?,
        config.web_server.bind_port,
        tx_request.clone(),
        rx_server_shutdown,
        history_cache.clone(),
    );

    let _ = persistence.lock().unwrap().flush();
    let _ = send_exit(&tx_request);
    let _ = modbus_handle.join();
    let _ = poller_handle.join();
    let _ = shutdown_dispatcher_handle.join();
    let _ = shutdown_handle.join();

    server_result
}
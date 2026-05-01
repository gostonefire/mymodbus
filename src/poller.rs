//! Data poller for Modbus registers
//!
//! This module periodically polls defined Modbus registers and stores the results in a history cache.

use anyhow::Result;
use log::{error, info};
use std::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::history_cache::HistoryCache;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};
use crate::persistence::Persistence;

/// A snapshot of power metrics at a specific point in time.
///
/// Power values are stored in their scaled real-world units, normally kW.
/// Battery SoC is stored as percent.
#[derive(Copy, Clone)]
pub struct PowerSample {
    /// Unix timestamp in seconds
    pub ts: u64,
    /// Power production in kW
    pub production: f64,
    /// Power consumption in kW
    pub consumption: f64,
    /// Battery state of charge, in percent
    pub batt_soc: f64,
}

/// Spawns a new poller thread
///
/// # Arguments
///
/// * `tx_request` - channel to send Modbus requests
/// * `rx_shutdown` - channel to receive shutdown signal
/// * `cache` - shared history cache to store samples
/// * `persistence` - shared persistence to store samples
/// * `production_id` - register ID for power production
/// * `consumption_id` - register ID for power consumption
/// * `batt_soc_id` - register ID for battery state of charge
pub fn spawn_poller(
    tx_request: mpsc::Sender<ModbusRequest>,
    rx_shutdown: mpsc::Receiver<()>,
    cache: Arc<HistoryCache>,
    persistence: Arc<Mutex<Persistence>>,
    production_id: String,
    consumption_id: String,
    batt_soc_id: String,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let interval = Duration::from_secs(60);
        let mut next_tick = Instant::now();

        loop {
            next_tick += interval;
            let now = Instant::now();

            if next_tick > now {
                match rx_shutdown.recv_timeout(next_tick - now) {
                    Ok(()) => {
                        info!("poller received shutdown signal");
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        info!("poller shutdown channel disconnected");
                        break;
                    }
                }
            } else {
                next_tick = Instant::now();
            }

            match poll_once(
                &tx_request,
                &cache,
                &persistence,
                &production_id,
                &consumption_id,
                &batt_soc_id,
            ) {
                Ok(()) => info!("polling cycle completed"),
                Err(err) => error!("polling cycle failed: {err}"),
            }
        }

        if let Err(err) = persistence.lock().unwrap().flush() {
            error!("failed to flush persistence during poller shutdown: {err}");
        }

        info!("poller stopped");
    })
}

/// Performs a single polling cycle
///
/// # Arguments
///
/// * `tx_request` - channel to send Modbus requests
/// * `cache` - history cache to store the result
/// * `persistence` - persistence to store the result
/// * `production_id` - register ID for power production
/// * `consumption_id` - register ID for power consumption
/// * `batt_soc_id` - register ID for battery state of charge
fn poll_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    cache: &HistoryCache,
    persistence: &Arc<Mutex<Persistence>>,
    production_id: &str,
    consumption_id: &str,
    batt_soc_id: &str,
) -> Result<()> {
    let production = send_request(tx_request, RegisterRequest::UniqueId(production_id.to_string()))?
        .to_f64()?;
    let consumption = send_request(tx_request, RegisterRequest::UniqueId(consumption_id.to_string()))?
        .to_f64()?;
    let batt_soc = send_request(tx_request, RegisterRequest::UniqueId(batt_soc_id.to_string()))?
        .to_f64()?;

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let sample = PowerSample {
        ts,
        production,
        consumption,
        batt_soc,
    };

    cache.insert(sample);
    persistence.lock().unwrap().append(sample)?;

    Ok(())
}
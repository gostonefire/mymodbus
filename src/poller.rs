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
/// Values are stored in kWh, unscaled. The standard scale for these values is 0.1 with
/// a precision of 1 decimal place.
///
#[derive(Copy, Clone)]
pub struct PowerSample {
    /// Unix timestamp in seconds
    pub ts: u64,
    /// Energy produced in kWh
    pub produced: u32,
    /// Energy consumed in kWh
    pub consumed: u32,
    /// Energy exported in kWh
    pub exported: u32,
    /// Battery state of charge, in percent
    pub batt_soc: u32,
}

/// Spawns a new poller thread
///
/// # Arguments
///
/// * `tx_request` - channel to send Modbus requests
/// * `rx_shutdown` - channel to receive shutdown signal
/// * `cache` - shared history cache to store samples
/// * `persistence` - shared persistence to store samples
/// * `produced_id` - register ID for produced energy
/// * `consumed_id` - register ID for consumed energy
/// * `exported_id` - register ID for exported energy
/// * `batt_soc_id` - register ID for battery state of charge
pub fn spawn_poller(
    tx_request: mpsc::Sender<ModbusRequest>,
    rx_shutdown: mpsc::Receiver<()>,
    cache: Arc<HistoryCache>,
    persistence: Arc<Mutex<Persistence>>,
    produced_id: String,
    consumed_id: String,
    exported_id: String,
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
                &produced_id,
                &consumed_id,
                &exported_id,
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
/// * `produced_id` - register ID for produced energy
/// * `consumed_id` - register ID for consumed energy
/// * `exported_id` - register ID for exported energy
/// * `batt_soc_id` - register ID for battery state of charge
fn poll_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    cache: &HistoryCache,
    persistence: &Arc<Mutex<Persistence>>,
    produced_id: &str,
    consumed_id: &str,
    exported_id: &str,
    batt_soc_id: &str,
) -> Result<()> {
    let produced = send_request(tx_request, RegisterRequest::UniqueId(produced_id.to_string()))?
        .to_u32()?;
    let consumed = send_request(tx_request, RegisterRequest::UniqueId(consumed_id.to_string()))?
        .to_u32()?;
    let exported = send_request(tx_request, RegisterRequest::UniqueId(exported_id.to_string()))?
        .to_u32()?;
    let batt_soc = send_request(tx_request, RegisterRequest::UniqueId(batt_soc_id.to_string()))?
        .to_u32()?;

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let sample = PowerSample {
        ts,
        produced,
        consumed,
        exported,
        batt_soc,
    };

    cache.insert(sample);
    persistence.lock().unwrap().append(sample)?;

    Ok(())
}
//! Data poller for Modbus registers
//!
//! This module periodically polls defined Modbus registers and stores the results in a history cache.

use anyhow::Result;
use log::{error, info};
use std::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::history_cache::HistoryCache;
use crate::latest_cache::LatestCache;
use crate::energy_interval_cache::{EnergyIntervalCache, EnergyTotalSample};
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};
use crate::persistence::Persistence;

const LATEST_ONLY_REGISTER_IDS: &[&str] = &[
    "battery_soh",
    "grid_power",
];

const FEED_IN_ENERGY_TOTAL_ID: &str = "feed_in_energy_total";
const GRID_CONSUMPTION_ENERGY_TOTAL_ID: &str = "grid_consumption_energy_total";
const FIFTEEN_MINUTES_SECS: u64 = 15 * 60;

/// A snapshot of some metrics at a specific point in time.
///
/// Power values are stored in their scaled real-world units, normally kW.
/// Battery SoC is stored as percent.
#[derive(Copy, Clone)]
pub struct DataSample {
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
/// * `latest_cache` - cache for latest-only values populated by the poller
/// * `energy_interval_cache` - cache for energy intervals values populated by the poller
/// * `persistence` - shared persistence to store samples
/// * `production_id` - register ID for power production
/// * `consumption_id` - register ID for power consumption
/// * `batt_soc_id` - register ID for battery state of charge
pub fn spawn_poller(
    tx_request: mpsc::Sender<ModbusRequest>,
    rx_shutdown: mpsc::Receiver<()>,
    cache: Arc<HistoryCache>,
    latest_cache: Arc<LatestCache>,
    energy_interval_cache: Arc<EnergyIntervalCache>,
    persistence: Arc<Mutex<Persistence>>,
    production_id: String,
    consumption_id: String,
    batt_soc_id: String,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let next_sample_ts = match next_minute_ts() {
                Ok(ts) => ts,
                Err(err) => {
                    error!("failed to calculate next poll timestamp: {err}");
                    break;
                }
            };

            let sleep_duration = match duration_until_unix_ts(next_sample_ts) {
                Ok(duration) => duration,
                Err(err) => {
                    error!("failed to calculate poll sleep duration: {err}");
                    break;
                }
            };

            match rx_shutdown.recv_timeout(sleep_duration) {
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

            match poll_once(
                &tx_request,
                &cache,
                &latest_cache,
                &energy_interval_cache,
                &persistence,
                &production_id,
                &consumption_id,
                &batt_soc_id,
                next_sample_ts,
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

/// Calculates the Unix timestamp of the start of the next minute
fn next_minute_ts() -> Result<u64> {
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    Ok(((now_ts / 60) + 1) * 60)
}

/// Calculates the duration from now until a given Unix timestamp
///
/// # Arguments
///
/// * `ts` - target Unix timestamp in seconds
fn duration_until_unix_ts(ts: u64) -> Result<Duration> {
    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    Ok(Duration::from_secs(ts.saturating_sub(now_ts)))
}

/// Performs a single polling cycle
///
/// # Arguments
///
/// * `tx_request` - channel to send Modbus requests
/// * `cache` - history cache to store the result
/// * `latest_cache` - cache for latest-only values populated by the poller
/// * `energy_interval_cache` - cache for energy intervals values populated by the poller
/// * `persistence` - persistence to store the result
/// * `production_id` - register ID for power production
/// * `consumption_id` - register ID for power consumption
/// * `batt_soc_id` - register ID for battery state of charge
/// * `ts` - timestamp for the sample
fn poll_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    cache: &HistoryCache,
    latest_cache: &LatestCache,
    energy_interval_cache: &EnergyIntervalCache,
    persistence: &Arc<Mutex<Persistence>>,
    production_id: &str,
    consumption_id: &str,
    batt_soc_id: &str,
    ts: u64,
) -> Result<()> {
    let production = send_request(tx_request, RegisterRequest::UniqueId(production_id.to_string()))?
        .to_f64()?;
    let consumption = send_request(tx_request, RegisterRequest::UniqueId(consumption_id.to_string()))?
        .to_f64()?;
    let batt_soc = send_request(tx_request, RegisterRequest::UniqueId(batt_soc_id.to_string()))?
        .to_f64()?;

    let sample = DataSample {
        ts,
        production,
        consumption,
        batt_soc,
    };

    cache.insert(sample);
    persistence.lock().unwrap().append(sample)?;

    latest_cache.insert(batt_soc_id, batt_soc, ts);
    latest_cache.insert(production_id, production, ts);
    latest_cache.insert(consumption_id, consumption, ts);

    for id in LATEST_ONLY_REGISTER_IDS {
        let value = send_request(tx_request, RegisterRequest::UniqueId((*id).to_string()))?
            .to_f64()?;
        latest_cache.insert(*id, value, ts);
    }

    if ts % FIFTEEN_MINUTES_SECS == 0 {
        poll_energy_totals_once(tx_request, latest_cache, energy_interval_cache, ts)?;
    }
    
    Ok(())
}

/// Polls energy total values once and updates caches
///
/// # Arguments
///
/// * `tx_request` - channel to send Modbus requests
/// * `latest_cache` - cache for latest values populated by the poller
/// * `energy_interval_cache` - cache for energy intervals values populated by the poller
/// * `ts` - timestamp for the sample
fn poll_energy_totals_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    latest_cache: &LatestCache,
    energy_interval_cache: &EnergyIntervalCache,
    ts: u64,
) -> Result<()> {
    let feed_in_energy_total = match send_request(
        tx_request,
        RegisterRequest::UniqueId(FEED_IN_ENERGY_TOTAL_ID.to_string()),
    )
        .and_then(|value| value.to_f64())
    {
        Ok(value) => value,
        Err(err) => {
            energy_interval_cache.clear();
            return Err(err);
        }
    };

    let grid_consumption_energy_total = match send_request(
        tx_request,
        RegisterRequest::UniqueId(GRID_CONSUMPTION_ENERGY_TOTAL_ID.to_string()),
    )
        .and_then(|value| value.to_f64())
    {
        Ok(value) => value,
        Err(err) => {
            energy_interval_cache.clear();
            return Err(err);
        }
    };

    latest_cache.insert(FEED_IN_ENERGY_TOTAL_ID, feed_in_energy_total, ts);
    latest_cache.insert(
        GRID_CONSUMPTION_ENERGY_TOTAL_ID,
        grid_consumption_energy_total,
        ts,
    );

    energy_interval_cache.insert(EnergyTotalSample {
        ts,
        feed_in_energy_total,
        grid_consumption_energy_total,
    });

    Ok(())
}
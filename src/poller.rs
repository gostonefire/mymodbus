use anyhow::Result;
use log::{error, info};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::history_cache::HistoryCache;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};

#[derive(Copy, Clone)]
pub struct PowerSample {
    pub ts: u64,
    pub produced: f64,
    pub consumed: f64,
    pub exported: f64,
}

pub fn spawn_poller(
    tx_request: mpsc::Sender<ModbusRequest>,
    rx_shutdown: mpsc::Receiver<()>,
    cache: Arc<HistoryCache>,
    produced_id: String,
    consumed_id: String,
    exported_id: String,
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

            match poll_once(&tx_request, &cache, &produced_id, &consumed_id, &exported_id) {
                Ok(()) => info!("polling cycle completed"),
                Err(err) => error!("polling cycle failed: {err}"),
            }
        }

        info!("poller stopped");
    })
}

fn poll_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    cache: &HistoryCache,
    produced_id: &str,
    consumed_id: &str,
    exported_id: &str,
) -> Result<()> {
    let produced = send_request(tx_request, RegisterRequest::UniqueId(produced_id.to_string()))?
        .to_f64()?;
    let consumed = send_request(tx_request, RegisterRequest::UniqueId(consumed_id.to_string()))?
        .to_f64()?;

    let exported = send_request(tx_request, RegisterRequest::UniqueId(exported_id.to_string()))?
        .to_f64()?;

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let sample = PowerSample {
        ts,
        produced,
        consumed,
        exported,
    };

    cache.insert(sample);
    Ok(())
}
use anyhow::Result;
use log::{error, info};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::history_cache::HistoryCache;
use crate::manager_modbus::{send_request, ModbusRequest, RegisterRequest};
use crate::persistence::{unix_now_secs, HistoryStore, PowerSample};

const SNAPSHOT_EVERY_SAMPLES: usize = 60;

pub fn spawn_poller(
    tx_request: mpsc::Sender<ModbusRequest>,
    store: HistoryStore,
    cache: Arc<HistoryCache>,
    produced_id: String,
    consumed_id: String,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let interval = Duration::from_secs(60);
        let mut next_tick = Instant::now();
        let mut samples_since_snapshot = 0usize;

        loop {
            next_tick += interval;
            let now = Instant::now();
            if next_tick > now {
                thread::sleep(next_tick - now);
            } else {
                next_tick = Instant::now();
            }

            match poll_once(&tx_request, &store, &cache, &produced_id, &consumed_id) {
                Ok(()) => {
                    samples_since_snapshot += 1;
                    info!("polling cycle completed");

                    if samples_since_snapshot >= SNAPSHOT_EVERY_SAMPLES {
                        match rotate_snapshot(&store) {
                            Ok(()) => {
                                samples_since_snapshot = 0;
                                info!("snapshot rotated successfully");
                            }
                            Err(err) => error!("snapshot rotation failed: {err}"),
                        }
                    }
                }
                Err(err) => error!("polling cycle failed: {err}"),
            }
        }
    })
}

fn poll_once(
    tx_request: &mpsc::Sender<ModbusRequest>,
    store: &HistoryStore,
    cache: &HistoryCache,
    produced_id: &str,
    consumed_id: &str,
) -> Result<()> {
    let produced = send_request(tx_request, RegisterRequest::UniqueId(produced_id.to_string()))?
        .to_f64()?;
    let consumed = send_request(tx_request, RegisterRequest::UniqueId(consumed_id.to_string()))?
        .to_f64()?;

    let sample = PowerSample {
        ts: unix_now_secs(),
        produced,
        consumed,
    };

    store.append_journal_batch(&[sample])?;
    cache.insert(sample);
    Ok(())
}

fn rotate_snapshot(store: &HistoryStore) -> Result<()> {
    let buffer = store.load()?;
    store.write_snapshot(&buffer)?;
    store.clear_journal()?;
    Ok(())
}

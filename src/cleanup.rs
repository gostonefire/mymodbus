//! This module handles the periodic cleanup of old persistence files.
//!

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use log::warn;
use crate::persistence::Persistence;

/// Spawns a worker thread that periodically cleans up old persistence files.
///
/// The worker runs every 24 hours and can be shut down using the provided receiver.
///
/// # Arguments
///
/// * `persistence` - A shared, thread-safe reference to the persistence manager.
/// * `rx_shutdown` - A receiver used to signal the worker to shut down.
pub fn spawn_cleanup_worker(
    persistence: Arc<Mutex<Persistence>>,
    rx_shutdown: mpsc::Receiver<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let interval = Duration::from_secs(24 * 60 * 60);

        loop {
            match rx_shutdown.recv_timeout(interval) {
                Ok(()) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Err(err) = persistence.lock().unwrap().cleanup_old_files() {
                        warn!("periodic persistence cleanup failed: {err}");
                    }
                }
            }
        }
    })
}

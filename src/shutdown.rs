use anyhow::Result;
use std::sync::mpsc::Sender;
use std::thread;

#[cfg(unix)]
use signal_hook::consts::signal::{SIGINT, SIGTERM};
#[cfg(unix)]
use signal_hook::iterator::Signals;

pub fn spawn_shutdown_listener(tx_shutdown: Sender<()>) -> Result<thread::JoinHandle<()>> {
    #[cfg(unix)]
    {
        let signals = Signals::new([SIGTERM, SIGINT])?;

        let handle = thread::spawn(move || {
            for _sig in signals.forever() {
                let _ = tx_shutdown.send(());
                break;
            }
        });

        Ok(handle)
    }

    #[cfg(windows)]
    {
        let (tx_ctrlc, rx_ctrlc) = std::sync::mpsc::channel::<()>();

        ctrlc::set_handler(move || {
            let _ = tx_ctrlc.send(());
        })?;

        let handle = thread::spawn(move || {
            if rx_ctrlc.recv().is_ok() {
                let _ = tx_shutdown.send(());
            }
        });

        Ok(handle)
    }
}
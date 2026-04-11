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

    #[cfg(not(unix))]
    {
        let handle = thread::spawn(move || {
            let _ = tx_shutdown.send(());
        });

        Ok(handle)
    }
}
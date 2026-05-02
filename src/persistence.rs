//! Append-only binary persistence for power samples.
//!
//! Files are rotated after a configured number of samples. On startup, all
//! available persistence files can be read back to rebuild the in-memory cache.
//!
//! The format is intentionally simple:
//!
//! File header:
//! - 8 bytes magic: b"MYMODB01"
//!
//! Repeated records:
//! - u64 timestamp, little endian
//! - f64 production, little endian
//! - f64 consumption, little endian
//! - f64 battery state of charge, little endian
//!
//! If the application crashes while writing, the last partial record is ignored
//! during restore.

use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::poller::DataSample;

const FILE_MAGIC: &[u8; 8] = b"MYMODB01";
const RECORD_SIZE: usize = 32;

/// Append-only sample persistence.
pub struct Persistence {
    /// Directory where persistence files are stored.
    directory: PathBuf,
    /// Maximum number of samples to store in a single file before rotation.
    max_samples_per_file: u64,
    /// Duration to keep persistence files before deletion.
    retention: Duration,
    /// Buffered writer for the current active persistence file.
    current_file: Option<BufWriter<File>>,
    /// Number of samples written to the current file.
    current_file_sample_count: u64,
    /// Path to the current active persistence file.
    current_file_path: Option<PathBuf>,
}

impl Persistence {
    /// Create persistence with explicit settings.
    ///
    /// # Arguments
    ///
    /// * `directory` - the directory to store persistence files
    /// * `max_samples_per_file` - the maximum number of samples per file
    /// * `retention` - the duration to keep samples
    pub fn new(
        directory: PathBuf,
        max_samples_per_file: u64,
        retention: Duration,
    ) -> Result<Self> {
        fs::create_dir_all(&directory)
            .with_context(|| format!("failed to create persistence directory {}", directory.display()))?;

        Ok(Self {
            directory,
            max_samples_per_file,
            retention,
            current_file: None,
            current_file_sample_count: 0,
            current_file_path: None,
        })
    }

    /// Append one sample to the current persistence file.
    ///
    /// This intentionally does not call `sync_all()` per sample, because that
    /// would create unnecessary SD-card wear. `flush()` pushes data out of the
    /// process buffer, while still allowing the OS to batch actual disk writes.
    ///
    /// # Arguments
    ///
    /// * `sample` - the power sample to persist
    pub fn append(&mut self, sample: DataSample) -> Result<()> {
        self.ensure_current_file()?;

        if self.current_file_sample_count >= self.max_samples_per_file {
            self.rotate()?;
        }

        let writer = self
            .current_file
            .as_mut()
            .context("persistence file not open")?;

        write_sample(writer, sample)?;
        writer.flush().context("failed to flush persistence sample")?;

        self.current_file_sample_count += 1;

        debug!(
            "persisted sample ts={}, current file samples={}",
            sample.ts,
            self.current_file_sample_count
        );

        Ok(())
    }

    /// Flush pending buffered data.
    ///
    /// Call this during graceful shutdown.
    ///
    pub fn flush(&mut self) -> Result<()> {
        if let Some(writer) = self.current_file.as_mut() {
            writer.flush().context("failed to flush persistence file")?;
        }

        Ok(())
    }

    /// Remove old persistence files.
    ///
    pub fn cleanup_old_files(&self) -> Result<()> {
        let now = SystemTime::now();

        for entry in self.persistence_files()? {
            let metadata = fs::metadata(&entry)
                .with_context(|| format!("failed to read metadata for {}", entry.display()))?;

            let Ok(modified) = metadata.modified() else {
                continue;
            };

            let Ok(age) = now.duration_since(modified) else {
                continue;
            };

            if age > self.retention {
                info!("removing old persistence file {}", entry.display());
                fs::remove_file(&entry)
                    .with_context(|| format!("failed to remove old persistence file {}", entry.display()))?;
            }
        }

        Ok(())
    }

    /// Load samples newer than `cutoff_ts`.
    ///
    /// The returned vector is sorted by timestamp.
    ///
    /// # Arguments
    ///
    /// * `cutoff_ts` - the Unix timestamp (in seconds) to load samples from
    pub fn load_since(&self, cutoff_ts: u64) -> Result<Vec<DataSample>> {
        let mut samples = Vec::new();

        for file in self.persistence_files()? {
            let loaded = read_samples_from_file(&file)
                .with_context(|| format!("failed to read persistence file {}", file.display()))?;

            samples.extend(loaded.into_iter().filter(|sample| sample.ts >= cutoff_ts));
        }

        samples.sort_by_key(|sample| sample.ts);
        Ok(samples)
    }

    /// Ensures that the current persistence file is open and ready for writing.
    ///
    fn ensure_current_file(&mut self) -> Result<()> {
        if self.current_file.is_some() {
            return Ok(());
        }

        let path = self.new_file_path()?;
        self.open_new_file(path)
    }

    /// Rotates the persistence file by closing the current one and opening a new one.
    ///
    fn rotate(&mut self) -> Result<()> {
        self.flush()?;
        self.current_file = None;
        self.current_file_sample_count = 0;
        self.current_file_path = None;

        let path = self.new_file_path()?;
        self.open_new_file(path)
    }

    /// Opens a new persistence file at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - the path where the new file should be created
    fn open_new_file(&mut self, path: PathBuf) -> Result<()> {
        info!("opening persistence file {}", path.display());

        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("failed to create persistence file {}", path.display()))?;

        file.write_all(FILE_MAGIC)
            .with_context(|| format!("failed to write persistence header {}", path.display()))?;

        self.current_file = Some(BufWriter::new(file));
        self.current_file_sample_count = 0;
        self.current_file_path = Some(path);

        Ok(())
    }

    /// Generates a unique path for a new persistence file based on the current system time.
    ///
    fn new_file_path(&self) -> Result<PathBuf> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX_EPOCH")?
            .as_secs();

        for suffix in 0..1_000_u32 {
            let name = if suffix == 0 {
                format!("samples-{}.bin", ts)
            } else {
                format!("samples-{}-{}.bin", ts, suffix)
            };

            let path = self.directory.join(name);

            if !path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!("could not allocate unique persistence file name");
    }

    /// Returns a list of all persistence files in the configured directory, sorted by name.
    ///
    fn persistence_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(&self.directory)
            .with_context(|| format!("failed to read persistence directory {}", self.directory.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if is_persistence_file(&path) {
                files.push(path);
            }
        }

        files.sort();

        Ok(files)
    }
}

/// Checks if a file path represents a valid persistence file.
///
/// # Arguments
///
/// * `path` - the path to check
fn is_persistence_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("samples-") && name.ends_with(".bin"))
        .unwrap_or(false)
}

/// Writes a single power sample to a writer in binary format.
///
/// # Arguments
///
/// * `writer` - the writer to write to
/// * `sample` - the sample to encode and write
fn write_sample<W: Write>(writer: &mut W, sample: DataSample) -> Result<()> {
    writer.write_all(&sample.ts.to_le_bytes())?;
    writer.write_all(&sample.production.to_le_bytes())?;
    writer.write_all(&sample.consumption.to_le_bytes())?;
    writer.write_all(&sample.batt_soc.to_le_bytes())?;
    Ok(())
}

/// Reads all power samples from a persistence file.
///
/// # Arguments
///
/// * `path` - the path to the persistence file
fn read_samples_from_file(path: &Path) -> Result<Vec<DataSample>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0_u8; 8];
    reader.read_exact(&mut magic)?;

    if &magic != FILE_MAGIC {
        warn!("ignoring persistence file with invalid header: {}", path.display());
        return Ok(Vec::new());
    }

    let mut samples = Vec::new();

    loop {
        let mut record = [0_u8; RECORD_SIZE];

        match reader.read_exact(&mut record) {
            Ok(()) => {
                samples.push(decode_sample(&record));
            }
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    Ok(samples)
}

/// Decodes a single power sample from its binary representation.
///
/// # Arguments
///
/// * `record` - the 20-byte record to decode
fn decode_sample(record: &[u8; RECORD_SIZE]) -> DataSample {
    let ts = u64::from_le_bytes(record[0..8].try_into().unwrap());
    let production = f64::from_le_bytes(record[8..16].try_into().unwrap());
    let consumption = f64::from_le_bytes(record[16..24].try_into().unwrap());
    let batt_soc = f64::from_le_bytes(record[24..32].try_into().unwrap());

    DataSample {
        ts,
        production,
        consumption,
        batt_soc,
    }
}
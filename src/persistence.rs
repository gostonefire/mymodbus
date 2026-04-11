use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// A single timestamped measurement.
///
/// This is reusable for any power-related metric or other time series value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedValue {
    /// Unix timestamp in seconds.
    pub ts: i64,
    /// Measured value.
    pub value: f64,
}

/// One historical record containing paired measurements.
///
/// At minimum this stores:
/// - power produced
/// - power consumed / load power
///
/// You can extend this later with more fields, such as exported power.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerSample {
    pub produced: TimedValue,
    pub consumed: TimedValue,
}

/// On-disk persistence for a sampled time series.
///
/// Strategy:
/// - Append new samples to a journal file
/// - Periodically write a full snapshot file
/// - On startup, load snapshot then replay journal
pub struct HistoryStore {
    snapshot_path: PathBuf,
    journal_path: PathBuf,
    max_samples: usize,
}

impl HistoryStore {
    pub fn new(
        snapshot_path: impl Into<PathBuf>,
        journal_path: impl Into<PathBuf>,
        max_samples: usize,
    ) -> Self {
        Self {
            snapshot_path: snapshot_path.into(),
            journal_path: journal_path.into(),
            max_samples,
        }
    }

    /// Load buffer contents from disk.
    ///
    /// Recovery order:
    /// 1. load snapshot
    /// 2. replay journal entries
    pub fn load(&self) -> Result<Vec<PowerSample>> {
        let mut buffer = Vec::with_capacity(self.max_samples);

        if self.snapshot_path.exists() {
            buffer = self
                .load_snapshot(&self.snapshot_path)
                .with_context(|| format!("loading snapshot {:?}", self.snapshot_path))?;
        }

        if self.journal_path.exists() {
            let journal_samples = self
                .load_journal(&self.journal_path)
                .with_context(|| format!("loading journal {:?}", self.journal_path))?;
            for sample in journal_samples {
                push_bounded(&mut buffer, sample, self.max_samples);
            }
        }

        Ok(buffer)
    }

    /// Append a batch of samples to the journal.
    ///
    /// Batch writes are much friendlier to flash storage than per-sample writes.
    pub fn append_journal_batch(&self, samples: &[PowerSample]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.journal_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating journal directory {:?}", parent))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)
            .with_context(|| format!("opening journal {:?}", self.journal_path))?;

        let mut writer = BufWriter::new(file);
        for sample in samples {
            write_sample(&mut writer, *sample)?;
        }
        writer.flush()?;
        Ok(())
    }

    /// Write the full buffer as a snapshot.
    ///
    /// This should be called infrequently, e.g. every hour or every few hours.
    pub fn write_snapshot(&self, buffer: &[PowerSample]) -> Result<()> {
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating snapshot directory {:?}", parent))?;
        }

        let tmp_path = self.snapshot_path.with_extension("snapshot.tmp");
        {
            let file = File::create(&tmp_path)
                .with_context(|| format!("creating temporary snapshot {:?}", tmp_path))?;
            let mut writer = BufWriter::new(file);

            write_u64(&mut writer, buffer.len() as u64)?;
            for sample in buffer {
                write_sample(&mut writer, *sample)?;
            }

            writer.flush()?;
        }

        fs::rename(&tmp_path, &self.snapshot_path)
            .with_context(|| format!("renaming {:?} to {:?}", tmp_path, self.snapshot_path))?;

        Ok(())
    }

    /// Compact the journal after a successful snapshot.
    ///
    /// A common pattern is:
    /// - write snapshot
    /// - replace journal with empty file
    pub fn clear_journal(&self) -> Result<()> {
        if let Some(parent) = self.journal_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating journal directory {:?}", parent))?;
        }

        let file = File::create(&self.journal_path)
            .with_context(|| format!("truncating journal {:?}", self.journal_path))?;
        file.sync_all()?;
        Ok(())
    }

    fn load_snapshot(&self, path: &Path) -> Result<Vec<PowerSample>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let count = read_u64(&mut reader)? as usize;
        let mut buffer = Vec::with_capacity(count.min(self.max_samples));

        for _ in 0..count {
            let sample = read_sample(&mut reader)?;
            push_bounded(&mut buffer, sample, self.max_samples);
        }

        Ok(buffer)
    }

    fn load_journal(&self, path: &Path) -> Result<Vec<PowerSample>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut buffer = Vec::new();
        loop {
            match read_sample(&mut reader) {
                Ok(sample) => buffer.push(sample),
                Err(err) => {
                    // EOF is expected at the end of the journal.
                    if is_eof(&err) {
                        break;
                    }
                    return Err(err);
                }
            }
        }

        Ok(buffer)
    }
}

/// Push while keeping only the most recent `max_samples`.
fn push_bounded(buffer: &mut Vec<PowerSample>, sample: PowerSample, max_samples: usize) {
    if buffer.len() == max_samples {
        buffer.remove(0);
    }
    buffer.push(sample);
}

fn write_sample<W: Write>(writer: &mut W, sample: PowerSample) -> Result<()> {
    write_timed_value(writer, sample.produced)?;
    write_timed_value(writer, sample.consumed)?;
    Ok(())
}

fn read_sample<R: Read>(reader: &mut R) -> Result<PowerSample> {
    Ok(PowerSample {
        produced: read_timed_value(reader)?,
        consumed: read_timed_value(reader)?,
    })
}

fn write_timed_value<W: Write>(writer: &mut W, item: TimedValue) -> Result<()> {
    write_i64(writer, item.ts)?;
    write_f64(writer, item.value)?;
    Ok(())
}

fn read_timed_value<R: Read>(reader: &mut R) -> Result<TimedValue> {
    Ok(TimedValue {
        ts: read_i64(reader)?,
        value: read_f64(reader)?,
    })
}

fn write_u64<W: Write>(writer: &mut W, value: u64) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_i64<R: Read>(reader: &mut R) -> Result<i64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn write_f64<W: Write>(writer: &mut W, value: f64) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_f64<R: Read>(reader: &mut R) -> Result<f64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn is_eof(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            io_err.kind() == std::io::ErrorKind::UnexpectedEof
        } else {
            false
        }
    })
}

/// Convenience helper if you want a timestamp source in this module.
pub fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
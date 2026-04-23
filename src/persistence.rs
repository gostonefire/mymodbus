use anyhow::{anyhow, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// A single sampled record with one timestamp for the whole poll cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerSample {
    /// Unix timestamp in seconds.
    pub ts: i64,
    /// Produced power value.
    pub produced: f64,
    /// Consumed/load power value.
    pub consumed: f64,
}

/// Helper for the current UNIX timestamp in seconds.
pub fn unix_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
    pub fn new<P1, P2>(snapshot_path: P1, journal_path: P2, max_samples: usize) -> Self
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
    {
        Self {
            snapshot_path: snapshot_path.into(),
            journal_path: journal_path.into(),
            max_samples,
        }
    }

    pub fn append_journal_batch(&self, samples: &[PowerSample]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.journal_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)?;

        let mut writer = BufWriter::new(file);
        for sample in samples {
            write_power_sample(&mut writer, *sample)?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn write_snapshot(&self, samples: &[PowerSample]) -> Result<()> {
        if let Some(parent) = self.snapshot_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(&self.snapshot_path)?;
        let mut writer = BufWriter::new(file);

        let count = samples.len().min(self.max_samples);
        write_u64(&mut writer, count as u64)?;
        for sample in samples.iter().take(count) {
            write_power_sample(&mut writer, *sample)?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn clear_journal(&self) -> Result<()> {
        if self.journal_path.exists() {
            File::create(&self.journal_path)?;
        }
        Ok(())
    }

    pub fn load(&self) -> Result<Vec<PowerSample>> {
        let mut buffer = Vec::new();

        if self.snapshot_path.exists() {
            let snapshot = read_power_sample_file(&self.snapshot_path)?;
            buffer.extend(snapshot);
        }

        if self.journal_path.exists() {
            let journal = read_power_sample_file(&self.journal_path)?;
            buffer.extend(journal);
        }

        if buffer.len() > self.max_samples {
            let start = buffer.len() - self.max_samples;
            buffer = buffer[start..].to_vec();
        }

        Ok(buffer)
    }
}

fn write_power_sample<W: Write>(writer: &mut W, item: PowerSample) -> Result<()> {
    write_i64(writer, item.ts)?;
    write_f64(writer, item.produced)?;
    write_f64(writer, item.consumed)?;
    Ok(())
}

fn read_power_sample<R: Read>(reader: &mut R) -> Result<PowerSample> {
    Ok(PowerSample {
        ts: read_i64(reader)?,
        produced: read_f64(reader)?,
        consumed: read_f64(reader)?,
    })
}

fn read_power_sample_file(path: &Path) -> Result<Vec<PowerSample>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let count = read_u64(&mut reader)? as usize;
    let mut samples = Vec::with_capacity(count);

    for _ in 0..count {
        samples.push(read_power_sample(&mut reader)?);
    }

    Ok(samples)
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

fn write_u64<W: Write>(writer: &mut W, value: u64) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
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

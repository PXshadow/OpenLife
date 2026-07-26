//! Efficient death event log: RAM ring → periodic disk journal.
//!
//! Line format (space-separated, single line):
//! `wall_ms p_id conn_id reason age food heat x y email`

use std::collections::VecDeque;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::death_cause::DeathCause;

#[derive(Debug, Clone)]
pub struct DeathRecord {
    pub wall_unix_ms: u64,
    pub p_id: i32,
    pub conn_id: u64,
    pub reason: String,
    pub age: f32,
    pub food: f32,
    /// Player heat / temperature at death (HX heat channel).
    pub heat: f32,
    pub x: i32,
    pub y: i32,
    pub email: String,
}

impl DeathRecord {
    pub fn to_journal_line(&self) -> String {
        format!(
            "{} {} {} {} {:.2} {:.2} {:.3} {} {} {}",
            self.wall_unix_ms,
            self.p_id,
            self.conn_id,
            self.reason.replace(' ', "_"),
            self.age,
            self.food,
            self.heat,
            self.x,
            self.y,
            self.email.replace(' ', "_")
        )
    }

    pub fn cause(&self) -> DeathCause {
        DeathCause::from_reason(&self.reason)
    }
}

/// In-memory death log with ~30s flush.
pub struct DeathLog {
    events: Mutex<VecDeque<DeathRecord>>,
    max_events: usize,
    path: PathBuf,
    last_flush: Mutex<Instant>,
    flush_interval: Duration,
    pub total: std::sync::atomic::AtomicU64,
    pub by_hunger: std::sync::atomic::AtomicU64,
    pub by_age: std::sync::atomic::AtomicU64,
    pub by_killed: std::sync::atomic::AtomicU64,
    pub by_suicide: std::sync::atomic::AtomicU64,
    pub by_other: std::sync::atomic::AtomicU64,
}

impl fmt::Debug for DeathLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeathLog")
    }
}

impl DeathLog {
    pub fn new(path: impl Into<PathBuf>, max_events: usize, flush_secs: u64) -> Self {
        Self {
            events: Mutex::new(VecDeque::with_capacity(max_events.min(2048))),
            max_events: max_events.max(32),
            path: path.into(),
            last_flush: Mutex::new(Instant::now()),
            flush_interval: Duration::from_secs(flush_secs.max(5)),
            total: std::sync::atomic::AtomicU64::new(0),
            by_hunger: std::sync::atomic::AtomicU64::new(0),
            by_age: std::sync::atomic::AtomicU64::new(0),
            by_killed: std::sync::atomic::AtomicU64::new(0),
            by_suicide: std::sync::atomic::AtomicU64::new(0),
            by_other: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn wall_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn record(&self, mut r: DeathRecord) {
        if r.wall_unix_ms == 0 {
            r.wall_unix_ms = Self::wall_ms();
        }
        use std::sync::atomic::Ordering::*;
        self.total.fetch_add(1, Relaxed);
        match DeathCause::from_reason(&r.reason) {
            DeathCause::Hunger => {
                self.by_hunger.fetch_add(1, Relaxed);
            }
            DeathCause::Age => {
                self.by_age.fetch_add(1, Relaxed);
            }
            DeathCause::Killed | DeathCause::KilledLegal => {
                self.by_killed.fetch_add(1, Relaxed);
            }
            DeathCause::Suicide => {
                self.by_suicide.fetch_add(1, Relaxed);
            }
            DeathCause::Unknown => {
                self.by_other.fetch_add(1, Relaxed);
            }
        }
        let mut g = self.events.lock().unwrap();
        if g.len() >= self.max_events {
            g.pop_front();
        }
        g.push_back(r);
    }

    pub fn needs_flush(&self) -> bool {
        self.last_flush
            .lock()
            .map(|t| t.elapsed() >= self.flush_interval)
            .unwrap_or(true)
    }

    pub fn flush_to_disk(&self) -> std::io::Result<usize> {
        let batch: Vec<DeathRecord> = {
            let mut g = self.events.lock().unwrap();
            g.drain(..).collect()
        };
        if batch.is_empty() {
            if let Ok(mut t) = self.last_flush.lock() {
                *t = Instant::now();
            }
            return Ok(0);
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if self.path.exists() {
            if let Ok(meta) = std::fs::metadata(&self.path) {
                if meta.len() > 4 * 1024 * 1024 {
                    let bak = PathBuf::from(format!("{}.1.bak", self.path.display()));
                    let _ = std::fs::remove_file(&bak);
                    let _ = std::fs::rename(&self.path, &bak);
                }
            }
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        for r in &batch {
            writeln!(f, "{}", r.to_journal_line())?;
        }
        if let Ok(mut t) = self.last_flush.lock() {
            *t = Instant::now();
        }
        info!(
            n = batch.len(),
            path = %self.path.display(),
            "death journal flushed"
        );
        Ok(batch.len())
    }

    pub fn try_flush(&self) {
        if !self.needs_flush() {
            return;
        }
        if let Err(e) = self.flush_to_disk() {
            warn!(error = %e, path = %self.path.display(), "death journal flush failed");
        }
    }

    pub fn summary_json(&self) -> serde_json::Value {
        use std::sync::atomic::Ordering::*;
        serde_json::json!({
            "total": self.total.load(Relaxed),
            "hunger": self.by_hunger.load(Relaxed),
            "age": self.by_age.load(Relaxed),
            "killed": self.by_killed.load(Relaxed),
            "suicide": self.by_suicide.load(Relaxed),
            "other": self.by_other.load(Relaxed),
            "buffered": self.events.lock().map(|g| g.len()).unwrap_or(0),
            "path": self.path.display().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_line_has_heat() {
        let r = DeathRecord {
            wall_unix_ms: 1,
            p_id: 9,
            conn_id: 1,
            reason: "reason_hunger".into(),
            age: 20.5,
            food: 0.0,
            heat: 0.42,
            x: 1,
            y: 2,
            email: "a@b".into(),
        };
        let line = r.to_journal_line();
        assert!(line.contains("0.420") || line.contains("0.42"));
        assert!(line.contains("reason_hunger"));
    }
}

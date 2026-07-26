//! Lightweight in-process metrics for ops and AI-driven evolution.
//!
//! ## `skip_ticks` meaning (Haxe parity — option A)
//! Extra tick-index advances for lag catch-up (compressed sim dt), **not**
//! tokio `MissedTickBehavior::Skip` dropped wakes.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Default)]
pub struct Counters {
    pub ticks: AtomicU64,
    pub intents_applied: AtomicU64,
    pub skip_ticks: AtomicU64,
    pub connections: AtomicU64,
    pub logins: AtomicU64,
    pub deaths: AtomicU64,
    pub crafts: AtomicU64,
    pub autosaves: AtomicU64,
    pub start_unix_ms: AtomicU64,
    pub tick_work_ema_us: AtomicU64,
    pub intent_ema_us: AtomicU64,
    pub lock_wait_ema_us: AtomicU64,
    pub selfplay_unstick_total: AtomicU64,
    pub ai_sim_time_ms: AtomicU64,
    pub ai_cpu_us: AtomicU64,
    pub ai_thinks: AtomicU64,
    /// Boot phase timings (ms) — set once at server start.
    pub boot_objects_ms: AtomicU64,
    pub boot_transitions_ms: AtomicU64,
    pub boot_world_ms: AtomicU64,
    pub boot_lineages_ms: AtomicU64,
    pub boot_accounts_ms: AtomicU64,
    pub boot_total_ms: AtomicU64,
    /// Latency window stats (µs) — avg / worst 10% / outlier counts.
    pub tick_work_avg_us: AtomicU64,
    pub tick_work_p90_us: AtomicU64,
    pub tick_work_outliers: AtomicU64,
    pub tick_work_normal: AtomicU64,
    pub intent_avg_us: AtomicU64,
    pub intent_p90_us: AtomicU64,
    pub intent_outliers: AtomicU64,
    pub intent_normal: AtomicU64,
    /// Human (real TCP client) intent latency window (µs).
    pub human_intent_avg_us: AtomicU64,
    pub human_intent_p90_us: AtomicU64,
    pub human_intent_count: AtomicU64,
    /// AI / self-play / NPC intent latency window (µs).
    pub ai_intent_avg_us: AtomicU64,
    pub ai_intent_p90_us: AtomicU64,
    pub ai_intent_count: AtomicU64,
}

impl Counters {
    pub const fn new() -> Self {
        Self {
            ticks: AtomicU64::new(0),
            intents_applied: AtomicU64::new(0),
            skip_ticks: AtomicU64::new(0),
            connections: AtomicU64::new(0),
            logins: AtomicU64::new(0),
            deaths: AtomicU64::new(0),
            crafts: AtomicU64::new(0),
            autosaves: AtomicU64::new(0),
            start_unix_ms: AtomicU64::new(0),
            tick_work_ema_us: AtomicU64::new(0),
            intent_ema_us: AtomicU64::new(0),
            lock_wait_ema_us: AtomicU64::new(0),
            selfplay_unstick_total: AtomicU64::new(0),
            ai_sim_time_ms: AtomicU64::new(0),
            ai_cpu_us: AtomicU64::new(0),
            ai_thinks: AtomicU64::new(0),
            boot_objects_ms: AtomicU64::new(0),
            boot_transitions_ms: AtomicU64::new(0),
            boot_world_ms: AtomicU64::new(0),
            boot_lineages_ms: AtomicU64::new(0),
            boot_accounts_ms: AtomicU64::new(0),
            boot_total_ms: AtomicU64::new(0),
            tick_work_avg_us: AtomicU64::new(0),
            tick_work_p90_us: AtomicU64::new(0),
            tick_work_outliers: AtomicU64::new(0),
            tick_work_normal: AtomicU64::new(0),
            intent_avg_us: AtomicU64::new(0),
            intent_p90_us: AtomicU64::new(0),
            intent_outliers: AtomicU64::new(0),
            intent_normal: AtomicU64::new(0),
            human_intent_avg_us: AtomicU64::new(0),
            human_intent_p90_us: AtomicU64::new(0),
            human_intent_count: AtomicU64::new(0),
            ai_intent_avg_us: AtomicU64::new(0),
            ai_intent_p90_us: AtomicU64::new(0),
            ai_intent_count: AtomicU64::new(0),
        }
    }

    /// Record one intent's wall time, split human vs AI by `conn_id` band.
    ///
    /// Self-play / NPC conn_ids are ≥ 9_000_000 (see ol-server selfplay/npc_ai).
    pub fn record_client_intent(&self, conn_id: u64, us: u64) {
        let is_ai = conn_id >= 9_000_000;
        if is_ai {
            self.ai_intent_count.fetch_add(1, Ordering::Relaxed);
            // EMA α=0.2
            let prev = self.ai_intent_avg_us.load(Ordering::Relaxed);
            let next = if prev == 0 {
                us
            } else {
                ((prev as f64) * 0.8 + (us as f64) * 0.2) as u64
            };
            self.ai_intent_avg_us.store(next, Ordering::Relaxed);
            let p90 = self.ai_intent_p90_us.load(Ordering::Relaxed);
            let p90n = if us > p90 {
                ((p90 as f64) * 0.7 + (us as f64) * 0.3) as u64
            } else {
                ((p90 as f64) * 0.95 + (us as f64) * 0.05) as u64
            };
            self.ai_intent_p90_us.store(p90n, Ordering::Relaxed);
        } else {
            self.human_intent_count.fetch_add(1, Ordering::Relaxed);
            let prev = self.human_intent_avg_us.load(Ordering::Relaxed);
            let next = if prev == 0 {
                us
            } else {
                ((prev as f64) * 0.8 + (us as f64) * 0.2) as u64
            };
            self.human_intent_avg_us.store(next, Ordering::Relaxed);
            let p90 = self.human_intent_p90_us.load(Ordering::Relaxed);
            let p90n = if us > p90 {
                ((p90 as f64) * 0.7 + (us as f64) * 0.3) as u64
            } else {
                ((p90 as f64) * 0.95 + (us as f64) * 0.05) as u64
            };
            self.human_intent_p90_us.store(p90n, Ordering::Relaxed);
        }
    }

    /// Record one-shot boot timings (only if total not already set).
    pub fn record_boot(
        &self,
        objects_ms: u64,
        transitions_ms: u64,
        world_ms: u64,
        lineages_ms: u64,
        accounts_ms: u64,
        total_ms: u64,
    ) {
        if self.boot_total_ms.load(Ordering::Relaxed) != 0 {
            return;
        }
        self.boot_objects_ms.store(objects_ms, Ordering::Relaxed);
        self.boot_transitions_ms
            .store(transitions_ms, Ordering::Relaxed);
        self.boot_world_ms.store(world_ms, Ordering::Relaxed);
        self.boot_lineages_ms.store(lineages_ms, Ordering::Relaxed);
        self.boot_accounts_ms.store(accounts_ms, Ordering::Relaxed);
        self.boot_total_ms.store(total_ms, Ordering::Relaxed);
    }

    pub fn mark_start_now(&self) {
        if self.start_unix_ms.load(Ordering::Relaxed) != 0 {
            return;
        }
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.start_unix_ms.store(ms, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            ticks: self.ticks.load(Ordering::Relaxed),
            intents_applied: self.intents_applied.load(Ordering::Relaxed),
            skip_ticks: self.skip_ticks.load(Ordering::Relaxed),
            connections: self.connections.load(Ordering::Relaxed),
            logins: self.logins.load(Ordering::Relaxed),
            deaths: self.deaths.load(Ordering::Relaxed),
            crafts: self.crafts.load(Ordering::Relaxed),
            autosaves: self.autosaves.load(Ordering::Relaxed),
            start_unix_ms: self.start_unix_ms.load(Ordering::Relaxed),
            tick_work_ema_us: self.tick_work_ema_us.load(Ordering::Relaxed),
            intent_ema_us: self.intent_ema_us.load(Ordering::Relaxed),
            lock_wait_ema_us: self.lock_wait_ema_us.load(Ordering::Relaxed),
            selfplay_unstick_total: self.selfplay_unstick_total.load(Ordering::Relaxed),
            ai_sim_time_ms: self.ai_sim_time_ms.load(Ordering::Relaxed),
            ai_cpu_us: self.ai_cpu_us.load(Ordering::Relaxed),
            ai_thinks: self.ai_thinks.load(Ordering::Relaxed),
            boot_objects_ms: self.boot_objects_ms.load(Ordering::Relaxed),
            boot_transitions_ms: self.boot_transitions_ms.load(Ordering::Relaxed),
            boot_world_ms: self.boot_world_ms.load(Ordering::Relaxed),
            boot_lineages_ms: self.boot_lineages_ms.load(Ordering::Relaxed),
            boot_accounts_ms: self.boot_accounts_ms.load(Ordering::Relaxed),
            boot_total_ms: self.boot_total_ms.load(Ordering::Relaxed),
            tick_work_avg_us: self.tick_work_avg_us.load(Ordering::Relaxed),
            tick_work_p90_us: self.tick_work_p90_us.load(Ordering::Relaxed),
            tick_work_outliers: self.tick_work_outliers.load(Ordering::Relaxed),
            tick_work_normal: self.tick_work_normal.load(Ordering::Relaxed),
            intent_avg_us: self.intent_avg_us.load(Ordering::Relaxed),
            intent_p90_us: self.intent_p90_us.load(Ordering::Relaxed),
            intent_outliers: self.intent_outliers.load(Ordering::Relaxed),
            intent_normal: self.intent_normal.load(Ordering::Relaxed),
            human_intent_avg_us: self.human_intent_avg_us.load(Ordering::Relaxed),
            human_intent_p90_us: self.human_intent_p90_us.load(Ordering::Relaxed),
            human_intent_count: self.human_intent_count.load(Ordering::Relaxed),
            ai_intent_avg_us: self.ai_intent_avg_us.load(Ordering::Relaxed),
            ai_intent_p90_us: self.ai_intent_p90_us.load(Ordering::Relaxed),
            ai_intent_count: self.ai_intent_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterSnapshot {
    pub ticks: u64,
    pub intents_applied: u64,
    pub skip_ticks: u64,
    pub connections: u64,
    pub logins: u64,
    pub deaths: u64,
    pub crafts: u64,
    pub autosaves: u64,
    pub start_unix_ms: u64,
    pub tick_work_ema_us: u64,
    pub intent_ema_us: u64,
    pub lock_wait_ema_us: u64,
    pub selfplay_unstick_total: u64,
    pub ai_sim_time_ms: u64,
    pub ai_cpu_us: u64,
    pub ai_thinks: u64,
    pub boot_objects_ms: u64,
    pub boot_transitions_ms: u64,
    pub boot_world_ms: u64,
    pub boot_lineages_ms: u64,
    pub boot_accounts_ms: u64,
    pub boot_total_ms: u64,
    pub tick_work_avg_us: u64,
    pub tick_work_p90_us: u64,
    pub tick_work_outliers: u64,
    pub tick_work_normal: u64,
    pub intent_avg_us: u64,
    pub intent_p90_us: u64,
    pub intent_outliers: u64,
    pub intent_normal: u64,
    pub human_intent_avg_us: u64,
    pub human_intent_p90_us: u64,
    pub human_intent_count: u64,
    pub ai_intent_avg_us: u64,
    pub ai_intent_p90_us: u64,
    pub ai_intent_count: u64,
}

/// Rolling latency window: average, worst ~10% (p90), outlier vs normal counts.
#[derive(Debug, Clone)]
pub struct LatencyWindow {
    samples: VecDeque<u32>,
    max: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencyStats {
    pub avg_us: u32,
    pub p90_us: u32,
    pub outliers: u32,
    pub normal: u32,
    pub n: u32,
}

impl LatencyWindow {
    pub fn new(max: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max.max(8)),
            max: max.max(8),
        }
    }

    pub fn record(&mut self, us: u32) {
        if self.samples.len() >= self.max {
            self.samples.pop_front();
        }
        self.samples.push_back(us);
    }

    pub fn stats(&self) -> LatencyStats {
        let n = self.samples.len();
        if n == 0 {
            return LatencyStats {
                avg_us: 0,
                p90_us: 0,
                outliers: 0,
                normal: 0,
                n: 0,
            };
        }
        let sum: u64 = self.samples.iter().map(|&v| v as u64).sum();
        let avg = (sum / n as u64) as u32;
        let mut sorted: Vec<u32> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        // Index of 90th percentile (worst 10% threshold).
        let idx = ((n as f32 - 1.0) * 0.9).round() as usize;
        let p90 = sorted[idx.min(n - 1)];
        // Count samples in the worst ~10% by rank (high latency tail).
        let top_start = (n * 9) / 10;
        let worst_n = (n - top_start).max(1) as u32;
        let outliers = worst_n;
        let normal = n as u32 - outliers;
        LatencyStats {
            avg_us: avg,
            p90_us: p90,
            outliers,
            normal,
            n: n as u32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmaLatency {
    alpha: f64,
    value_secs: f64,
    samples: u64,
}

impl EmaLatency {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            value_secs: 0.0,
            samples: 0,
        }
    }

    pub fn record(&mut self, d: Duration) {
        let s = d.as_secs_f64();
        if self.samples == 0 {
            self.value_secs = s;
        } else {
            self.value_secs = self.alpha * s + (1.0 - self.alpha) * self.value_secs;
        }
        self.samples += 1;
    }

    pub fn secs(&self) -> f64 {
        self.value_secs
    }

    pub fn micros(&self) -> u64 {
        (self.value_secs * 1_000_000.0).round() as u64
    }

    pub fn samples(&self) -> u64 {
        self.samples
    }
}

pub struct ScopeTimer {
    start: Instant,
}

impl ScopeTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpsSample {
    pub wall_unix_ms: u64,
    pub tick: u64,
    pub skip_ticks: u64,
    pub tick_work_us: u32,
    pub intent_ema_us: u32,
    pub lock_wait_ema_us: u32,
    pub intents: u64,
    pub connections: u64,
    /// Average tick work (µs) over latency window.
    pub tick_avg_us: u32,
    /// ~p90 tick work (µs) — threshold for worst ~10%.
    pub tick_p90_us: u32,
    pub tick_outliers: u32,
    pub tick_normal: u32,
    pub intent_avg_us: u32,
    pub intent_p90_us: u32,
    pub intent_outliers: u32,
    pub intent_normal: u32,
    /// Boot timings only meaningful on first sample after start (else 0).
    pub boot_total_ms: u32,
}

#[derive(Debug)]
pub struct OpsSeries {
    pub samples: VecDeque<OpsSample>,
    pub sample_every_ticks: u64,
    pub last_flush: Instant,
    pub flush_interval: Duration,
    pub intent_ema: EmaLatency,
    pub tick_ema: EmaLatency,
    pub lock_wait_ema: EmaLatency,
    pub tick_window: LatencyWindow,
    pub intent_window: LatencyWindow,
    pub max_samples: usize,
}

impl Default for OpsSeries {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(300), 360)
    }
}

impl OpsSeries {
    pub fn new(sample_every_ticks: u64, flush_interval: Duration, max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples.min(360)),
            sample_every_ticks: sample_every_ticks.max(1),
            last_flush: Instant::now(),
            flush_interval,
            intent_ema: EmaLatency::new(0.2),
            tick_ema: EmaLatency::new(0.2),
            lock_wait_ema: EmaLatency::new(0.2),
            tick_window: LatencyWindow::new(200),
            intent_window: LatencyWindow::new(200),
            max_samples: max_samples.max(1),
        }
    }

    pub fn on_tick_work(&mut self, d: Duration) {
        self.tick_ema.record(d);
        let us = d.as_micros().min(u128::from(u32::MAX)) as u32;
        self.tick_window.record(us);
    }

    pub fn on_intent(&mut self, d: Duration) {
        self.intent_ema.record(d);
        let us = d.as_micros().min(u128::from(u32::MAX)) as u32;
        self.intent_window.record(us);
    }

    pub fn on_lock_wait(&mut self, d: Duration) {
        self.lock_wait_ema.record(d);
    }

    pub fn maybe_sample(&mut self, tick: u64, counters: &Counters) {
        counters
            .tick_work_ema_us
            .store(self.tick_ema.micros(), Ordering::Relaxed);
        counters
            .intent_ema_us
            .store(self.intent_ema.micros(), Ordering::Relaxed);
        counters
            .lock_wait_ema_us
            .store(self.lock_wait_ema.micros(), Ordering::Relaxed);

        if tick == 0 || tick % self.sample_every_ticks != 0 {
            return;
        }
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let snap = counters.snapshot();
        let tw = self.tick_window.stats();
        let iw = self.intent_window.stats();
        counters
            .tick_work_avg_us
            .store(tw.avg_us as u64, Ordering::Relaxed);
        counters
            .tick_work_p90_us
            .store(tw.p90_us as u64, Ordering::Relaxed);
        counters
            .tick_work_outliers
            .store(tw.outliers as u64, Ordering::Relaxed);
        counters
            .tick_work_normal
            .store(tw.normal as u64, Ordering::Relaxed);
        counters
            .intent_avg_us
            .store(iw.avg_us as u64, Ordering::Relaxed);
        counters
            .intent_p90_us
            .store(iw.p90_us as u64, Ordering::Relaxed);
        counters
            .intent_outliers
            .store(iw.outliers as u64, Ordering::Relaxed);
        counters
            .intent_normal
            .store(iw.normal as u64, Ordering::Relaxed);
        let sample = OpsSample {
            wall_unix_ms: wall,
            tick: snap.ticks,
            skip_ticks: snap.skip_ticks,
            tick_work_us: self.tick_ema.micros().min(u32::MAX as u64) as u32,
            intent_ema_us: self.intent_ema.micros().min(u32::MAX as u64) as u32,
            lock_wait_ema_us: self.lock_wait_ema.micros().min(u32::MAX as u64) as u32,
            intents: snap.intents_applied,
            connections: snap.connections,
            tick_avg_us: tw.avg_us,
            tick_p90_us: tw.p90_us,
            tick_outliers: tw.outliers,
            tick_normal: tw.normal,
            intent_avg_us: iw.avg_us,
            intent_p90_us: iw.p90_us,
            intent_outliers: iw.outliers,
            intent_normal: iw.normal,
            boot_total_ms: snap.boot_total_ms.min(u32::MAX as u64) as u32,
        };
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn needs_flush(&self) -> bool {
        self.last_flush.elapsed() >= self.flush_interval
    }

    pub fn samples_since(&self, since_wall_ms: u64) -> Vec<OpsSample> {
        self.samples
            .iter()
            .copied()
            .filter(|s| s.wall_unix_ms > since_wall_ms)
            .collect()
    }

    pub fn snapshot_samples(&self) -> Vec<OpsSample> {
        self.samples.iter().copied().collect()
    }

    pub fn mark_flushed(&mut self) {
        self.last_flush = Instant::now();
    }

    pub fn take_flush_batch(&mut self) -> Vec<OpsSample> {
        self.mark_flushed();
        self.snapshot_samples()
    }
}

pub const OPS_JOURNAL_MAX_BYTES: u64 = 8 * 1024 * 1024;

pub fn format_ops_journal_line(s: &OpsSample) -> String {
    format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        s.wall_unix_ms,
        s.tick,
        s.skip_ticks,
        s.tick_work_us,
        s.intent_ema_us,
        s.lock_wait_ema_us,
        s.intents,
        s.connections,
        s.tick_avg_us,
        s.tick_p90_us,
        s.tick_outliers,
        s.tick_normal,
        s.intent_avg_us,
        s.intent_p90_us,
        s.intent_outliers,
        s.intent_normal,
        s.boot_total_ms
    )
}

pub fn maybe_rotate_ops_journal(path: &Path, max_bytes: u64) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let len = std::fs::metadata(path)?.len();
    if len < max_bytes {
        return Ok(());
    }
    let bak = PathBuf::from(format!("{}.1.bak", path.display()));
    let _ = std::fs::remove_file(&bak);
    std::fs::rename(path, &bak)?;
    Ok(())
}

pub fn append_ops_journal(path: &Path, samples: &[OpsSample]) -> std::io::Result<()> {
    use std::io::Write;
    if samples.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    maybe_rotate_ops_journal(path, OPS_JOURNAL_MAX_BYTES)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for s in samples {
        writeln!(f, "{}", format_ops_journal_line(s))?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Health {
    pub ok: bool,
    pub tick: u64,
    pub skip_ticks: u64,
    pub connections: u64,
    pub logins: u64,
    pub deaths: u64,
    pub crafts: u64,
    pub autosaves: u64,
    pub version: &'static str,
}

impl Health {
    pub fn from_counters(c: &Counters, version: &'static str) -> Self {
        let s = c.snapshot();
        Self {
            ok: true,
            tick: s.ticks,
            skip_ticks: s.skip_ticks,
            connections: s.connections,
            logins: s.logins,
            deaths: s.deaths,
            crafts: s.crafts,
            autosaves: s.autosaves,
            version,
        }
    }

    pub fn to_json_line(&self) -> String {
        format!(
            "{{\"ok\":{},\"tick\":{},\"skip_ticks\":{},\"connections\":{},\"logins\":{},\"deaths\":{},\"crafts\":{},\"autosaves\":{},\"version\":\"{}\"}}",
            self.ok,
            self.tick,
            self.skip_ticks,
            self.connections,
            self.logins,
            self.deaths,
            self.crafts,
            self.autosaves,
            self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_roundtrip() {
        let c = Counters::new();
        c.ticks.fetch_add(3, Ordering::Relaxed);
        assert_eq!(c.snapshot().ticks, 3);
    }

    #[test]
    fn login_death_craft_counters() {
        let c = Counters::new();
        c.logins.fetch_add(2, Ordering::Relaxed);
        c.deaths.fetch_add(1, Ordering::Relaxed);
        c.crafts.fetch_add(4, Ordering::Relaxed);
        c.autosaves.fetch_add(3, Ordering::Relaxed);
        let s = c.snapshot();
        assert_eq!(s.logins, 2);
        assert_eq!(s.deaths, 1);
        assert_eq!(s.crafts, 4);
        assert_eq!(s.autosaves, 3);
    }

    #[test]
    fn health_json() {
        let c = Counters::new();
        let h = Health::from_counters(&c, "0.1.0");
        assert!(h.to_json_line().contains("\"ok\":true"));
    }

    #[test]
    fn ema_records() {
        let mut e = EmaLatency::new(0.5);
        e.record(Duration::from_millis(10));
        e.record(Duration::from_millis(20));
        assert!(e.secs() > 0.0);
        assert_eq!(e.samples(), 2);
    }

    #[test]
    fn ops_series_samples_and_cap() {
        let mut ops = OpsSeries::new(1, Duration::from_secs(300), 3);
        let c = Counters::new();
        for t in 1..=5 {
            c.ticks.store(t, Ordering::Relaxed);
            ops.on_tick_work(Duration::from_micros(100 * t));
            ops.maybe_sample(t, &c);
        }
        assert_eq!(ops.samples.len(), 3);
        assert!(c.tick_work_ema_us.load(Ordering::Relaxed) > 0);
        let line = format_ops_journal_line(ops.samples.front().unwrap());
        assert!(line.split_whitespace().count() >= 16);
        assert!(c.tick_work_avg_us.load(Ordering::Relaxed) > 0 || c.tick_work_ema_us.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn ops_samples_since_delta() {
        let mut ops = OpsSeries::new(1, Duration::from_secs(300), 10);
        for (i, wall) in [(1u64, 1000u64), (2, 2000), (3, 3000), (4, 4000)] {
            ops.samples.push_back(OpsSample {
                wall_unix_ms: wall,
                tick: i,
                skip_ticks: 0,
                tick_work_us: 10,
                intent_ema_us: 5,
                lock_wait_ema_us: 0,
                intents: i,
                connections: 1,
                tick_avg_us: 10,
                tick_p90_us: 12,
                tick_outliers: 1,
                tick_normal: 9,
                intent_avg_us: 5,
                intent_p90_us: 6,
                intent_outliers: 1,
                intent_normal: 9,
                boot_total_ms: 0,
            });
        }
        let delta = ops.samples_since(2000);
        assert_eq!(delta.len(), 2);
        assert_eq!(delta[0].tick, 3);
        assert_eq!(delta[1].tick, 4);
        let line = format_ops_journal_line(&delta[0]);
        assert!(line.split_whitespace().count() >= 16);
    }

    #[test]
    fn latency_window_avg_p90_outliers() {
        let mut w = LatencyWindow::new(20);
        for i in 1..=10 {
            w.record(i * 10);
        }
        let s = w.stats();
        assert_eq!(s.n, 10);
        assert_eq!(s.avg_us, 55); // (10+20+...+100)/10 = 55
        assert!(s.p90_us >= 90);
        assert_eq!(s.outliers + s.normal, 10);
        assert!(s.outliers >= 1);
    }

    #[test]
    fn ops_sample_every_cadence() {
        let mut ops = OpsSeries::new(2, Duration::from_secs(300), 10);
        let c = Counters::new();
        for t in 1..=5 {
            c.ticks.store(t, Ordering::Relaxed);
            ops.maybe_sample(t, &c);
        }
        assert_eq!(ops.samples.len(), 2);
        assert_eq!(ops.samples[0].tick, 2);
        assert_eq!(ops.samples[1].tick, 4);
    }

    #[test]
    fn append_ops_journal_tempfile() {
        let dir = std::env::temp_dir().join(format!(
            "ol_ops_j_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ops_metrics.journal");
        let s = sample_zero(100, 7);
        append_ops_journal(&path, &[s]).unwrap();
        append_ops_journal(&path, &[s]).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn sample_zero(wall: u64, tick: u64) -> OpsSample {
        OpsSample {
            wall_unix_ms: wall,
            tick,
            skip_ticks: 0,
            tick_work_us: 10,
            intent_ema_us: 5,
            lock_wait_ema_us: 0,
            intents: 1,
            connections: 1,
            tick_avg_us: 10,
            tick_p90_us: 12,
            tick_outliers: 1,
            tick_normal: 9,
            intent_avg_us: 5,
            intent_p90_us: 6,
            intent_outliers: 1,
            intent_normal: 9,
            boot_total_ms: 0,
        }
    }

    #[test]
    fn maybe_rotate_ops_journal_renames_when_over_max() {
        let dir = std::env::temp_dir().join(format!(
            "ol_ops_rot_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ops_metrics.journal");
        std::fs::write(&path, vec![b'x'; 64]).unwrap();
        maybe_rotate_ops_journal(&path, 32).unwrap();
        assert!(!path.exists());
        let bak = PathBuf::from(format!("{}.1.bak", path.display()));
        assert!(bak.exists());
        let s = sample_zero(1, 1);
        append_ops_journal(&path, &[s]).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_ticks_docs_not_tokio() {
        let c = Counters::new();
        assert_eq!(c.snapshot().skip_ticks, 0);
        c.skip_ticks.fetch_add(1, Ordering::Relaxed);
        assert_eq!(c.snapshot().skip_ticks, 1);
    }
}

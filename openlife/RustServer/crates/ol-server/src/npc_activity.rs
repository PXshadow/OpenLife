//! In-memory NPC activity log with periodic disk flush (default 30s).
//!
//! Tracks what NPCs attempt (craft/eat/move/combat), time spent, stuck loops,
//! death age/reason. No per-event disk I/O.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Kind of NPC decision / outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcActivityKind {
    Think,
    Move,
    Craft,
    CraftPlan,
    Eat,
    SeekFood,
    Explore,
    Combat,
    Feed,
    Stuck,
    StuckCycle,
    Death,
    Error,
}

impl NpcActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Think => "think",
            Self::Move => "move",
            Self::Craft => "craft",
            Self::CraftPlan => "craft_plan",
            Self::Eat => "eat",
            Self::SeekFood => "seek_food",
            Self::Explore => "explore",
            Self::Combat => "combat",
            Self::Feed => "feed",
            Self::Stuck => "stuck",
            Self::StuckCycle => "stuck_cycle",
            Self::Death => "death",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpcActivityEvent {
    pub wall_unix_ms: u64,
    pub conn_id: u64,
    pub p_id: i32,
    pub kind: NpcActivityKind,
    /// Wall CPU micros for this decision slice.
    pub cpu_us: u32,
    /// Estimated sim/game time cost for the action (ms).
    pub game_ms: u32,
    pub age: f32,
    pub food: f32,
    pub x: i32,
    pub y: i32,
    pub held_id: i32,
    /// Optional detail (craft target, death reason, stuck transition…).
    pub detail: String,
}

impl NpcActivityEvent {
    pub fn to_journal_line(&self) -> String {
        format!(
            "{} {} {} {} {} {} {:.2} {:.1} {} {} {} {}",
            self.wall_unix_ms,
            self.conn_id,
            self.p_id,
            self.kind.as_str(),
            self.cpu_us,
            self.game_ms,
            self.age,
            self.food,
            self.x,
            self.y,
            self.held_id,
            self.detail.replace('\n', " ").replace('\r', "")
        )
    }
}

/// Per-NPC stuck / cycle tracker (pure, no I/O).
#[derive(Debug, Default, Clone)]
pub struct NpcStuckTracker {
    pub last_x: i32,
    pub last_y: i32,
    pub last_held: i32,
    pub last_detail: String,
    pub same_pos_count: u32,
    pub same_action_count: u32,
    /// Recent positions for cycle detection (A→B→A).
    pub pos_ring: VecDeque<(i32, i32)>,
    /// Recent craft keys "actor+target".
    pub craft_ring: VecDeque<String>,
    pub was_deleted: bool,
}

impl NpcStuckTracker {
    pub fn note_position(&mut self, x: i32, y: i32) {
        if x == self.last_x && y == self.last_y {
            self.same_pos_count = self.same_pos_count.saturating_add(1);
        } else {
            self.same_pos_count = 0;
            self.last_x = x;
            self.last_y = y;
        }
        self.pos_ring.push_back((x, y));
        if self.pos_ring.len() > 8 {
            self.pos_ring.pop_front();
        }
    }

    pub fn note_action(&mut self, detail: &str) {
        if detail == self.last_detail && !detail.is_empty() {
            self.same_action_count = self.same_action_count.saturating_add(1);
        } else {
            self.same_action_count = 0;
            self.last_detail = detail.to_string();
        }
    }

    pub fn note_craft_key(&mut self, key: String) {
        self.craft_ring.push_back(key);
        if self.craft_ring.len() > 6 {
            self.craft_ring.pop_front();
        }
    }

    /// True if last 4 positions oscillate A B A B.
    pub fn position_cycle(&self) -> bool {
        if self.pos_ring.len() < 4 {
            return false;
        }
        let n = self.pos_ring.len();
        let a = self.pos_ring[n - 4];
        let b = self.pos_ring[n - 3];
        let c = self.pos_ring[n - 2];
        let d = self.pos_ring[n - 1];
        a == c && b == d && a != b
    }

    /// True if same craft key five times in a row (allows multi-step walks).
    pub fn craft_loop(&self) -> bool {
        if self.craft_ring.len() < 5 {
            return false;
        }
        let n = self.craft_ring.len();
        let last = &self.craft_ring[n - 1];
        self.craft_ring.iter().rev().take(5).all(|k| k == last)
    }

    pub fn is_stuck(&self) -> bool {
        self.same_pos_count >= 12
            || self.same_action_count >= 8
            || self.position_cycle()
            || self.craft_loop()
    }
}

pub struct NpcActivityLog {
    events: Mutex<VecDeque<NpcActivityEvent>>,
    max_events: usize,
    path: PathBuf,
    last_flush: Mutex<Instant>,
    flush_interval: Duration,
    /// Aggregates for quick counters (reset on flush snapshot optional).
    pub craft_attempts: std::sync::atomic::AtomicU64,
    pub eat_attempts: std::sync::atomic::AtomicU64,
    pub stuck_events: std::sync::atomic::AtomicU64,
    pub deaths: std::sync::atomic::AtomicU64,
    pub cpu_us_total: std::sync::atomic::AtomicU64,
    pub game_ms_craft: std::sync::atomic::AtomicU64,
    pub game_ms_eat: std::sync::atomic::AtomicU64,
    pub game_ms_move: std::sync::atomic::AtomicU64,
    pub game_ms_other: std::sync::atomic::AtomicU64,
}

impl NpcActivityLog {
    pub fn new(path: impl Into<PathBuf>, max_events: usize, flush_secs: u64) -> Self {
        Self {
            events: Mutex::new(VecDeque::with_capacity(max_events.min(4096))),
            max_events: max_events.max(64),
            path: path.into(),
            last_flush: Mutex::new(Instant::now()),
            flush_interval: Duration::from_secs(flush_secs.max(5)),
            craft_attempts: std::sync::atomic::AtomicU64::new(0),
            eat_attempts: std::sync::atomic::AtomicU64::new(0),
            stuck_events: std::sync::atomic::AtomicU64::new(0),
            deaths: std::sync::atomic::AtomicU64::new(0),
            cpu_us_total: std::sync::atomic::AtomicU64::new(0),
            game_ms_craft: std::sync::atomic::AtomicU64::new(0),
            game_ms_eat: std::sync::atomic::AtomicU64::new(0),
            game_ms_move: std::sync::atomic::AtomicU64::new(0),
            game_ms_other: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from("SaveFiles/npc_activity.journal")
    }

    fn wall_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn push(&self, mut ev: NpcActivityEvent) {
        // Surface decisions in ol-server.log so operators can follow AI live.
        info!(
            conn_id = ev.conn_id,
            p_id = ev.p_id,
            kind = ev.kind.as_str(),
            age = format!("{:.1}", ev.age),
            food = format!("{:.1}", ev.food),
            x = ev.x,
            y = ev.y,
            held = ev.held_id,
            detail = %ev.detail,
            "npc activity"
        );
        if ev.wall_unix_ms == 0 {
            ev.wall_unix_ms = Self::wall_ms();
        }
        use std::sync::atomic::Ordering::*;
        self.cpu_us_total
            .fetch_add(ev.cpu_us as u64, Relaxed);
        match ev.kind {
            NpcActivityKind::Craft | NpcActivityKind::CraftPlan => {
                self.craft_attempts.fetch_add(1, Relaxed);
                self.game_ms_craft
                    .fetch_add(ev.game_ms as u64, Relaxed);
            }
            NpcActivityKind::Eat | NpcActivityKind::SeekFood => {
                self.eat_attempts.fetch_add(1, Relaxed);
                self.game_ms_eat.fetch_add(ev.game_ms as u64, Relaxed);
            }
            NpcActivityKind::Move | NpcActivityKind::Explore => {
                self.game_ms_move.fetch_add(ev.game_ms as u64, Relaxed);
            }
            NpcActivityKind::Stuck | NpcActivityKind::StuckCycle => {
                self.stuck_events.fetch_add(1, Relaxed);
                self.game_ms_other
                    .fetch_add(ev.game_ms as u64, Relaxed);
            }
            NpcActivityKind::Death => {
                self.deaths.fetch_add(1, Relaxed);
                self.game_ms_other
                    .fetch_add(ev.game_ms as u64, Relaxed);
            }
            _ => {
                self.game_ms_other
                    .fetch_add(ev.game_ms as u64, Relaxed);
            }
        }
        let mut g = self.events.lock().unwrap();
        if g.len() >= self.max_events {
            g.pop_front();
        }
        g.push_back(ev);
    }

    pub fn needs_flush(&self) -> bool {
        self.last_flush
            .lock()
            .map(|t| t.elapsed() >= self.flush_interval)
            .unwrap_or(true)
    }

    /// Drain a batch for writing (does not clear if write fails — caller drains after success).
    pub fn snapshot_and_clear(&self) -> Vec<NpcActivityEvent> {
        let mut g = self.events.lock().unwrap();
        g.drain(..).collect()
    }

    pub fn flush_to_disk(&self) -> std::io::Result<usize> {
        let batch = self.snapshot_and_clear();
        if batch.is_empty() {
            if let Ok(mut t) = self.last_flush.lock() {
                *t = Instant::now();
            }
            return Ok(0);
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Rotate if huge.
        if self.path.exists() {
            if let Ok(meta) = std::fs::metadata(&self.path) {
                if meta.len() > 8 * 1024 * 1024 {
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
        for ev in &batch {
            writeln!(f, "{}", ev.to_journal_line())?;
        }
        if let Ok(mut t) = self.last_flush.lock() {
            *t = Instant::now();
        }
        info!(
            n = batch.len(),
            path = %self.path.display(),
            "npc activity journal flushed"
        );
        Ok(batch.len())
    }

    pub fn try_flush(&self) {
        if !self.needs_flush() {
            return;
        }
        if let Err(e) = self.flush_to_disk() {
            warn!(error = %e, path = %self.path.display(), "npc activity flush failed");
        }
    }

    pub fn summary_json(&self) -> serde_json::Value {
        use std::sync::atomic::Ordering::*;
        serde_json::json!({
            "craft_attempts": self.craft_attempts.load(Relaxed),
            "eat_attempts": self.eat_attempts.load(Relaxed),
            "stuck_events": self.stuck_events.load(Relaxed),
            "deaths": self.deaths.load(Relaxed),
            "cpu_us_total": self.cpu_us_total.load(Relaxed),
            "game_ms_craft": self.game_ms_craft.load(Relaxed),
            "game_ms_eat": self.game_ms_eat.load(Relaxed),
            "game_ms_move": self.game_ms_move.load(Relaxed),
            "game_ms_other": self.game_ms_other.load(Relaxed),
            "buffered": self.events.lock().map(|g| g.len()).unwrap_or(0),
            "path": self.path.display().to_string(),
        })
    }
}

/// Append helper used by tests without full log.
pub fn format_header_comment() -> &'static str {
    "# wall_ms conn_id p_id kind cpu_us game_ms age food x y held detail"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stuck_cycle_detection() {
        let mut t = NpcStuckTracker::default();
        for &(x, y) in &[(0, 0), (1, 0), (0, 0), (1, 0)] {
            t.note_position(x, y);
        }
        assert!(t.position_cycle());
    }

    #[test]
    fn craft_loop_detection() {
        let mut t = NpcStuckTracker::default();
        // craft_loop requires five identical keys in a row (allows multi-step walks).
        for _ in 0..5 {
            t.note_craft_key("0+36".into());
        }
        assert!(t.craft_loop());
    }

    #[test]
    fn journal_line_single_line() {
        let ev = NpcActivityEvent {
            wall_unix_ms: 1,
            conn_id: 9,
            p_id: 1,
            kind: NpcActivityKind::Craft,
            cpu_us: 100,
            game_ms: 500,
            age: 14.0,
            food: 8.0,
            x: 1,
            y: 2,
            held_id: 0,
            detail: "want=404 a=0 t=36".into(),
        };
        let line = ev.to_journal_line();
        assert!(!line.contains('\n'));
        assert!(line.contains("craft"));
    }
}

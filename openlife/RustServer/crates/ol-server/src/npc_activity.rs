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
    /// Remaining seconds before Haxe `ServerAi.doRebirth` respawn (0 = ready).
    pub rebirth_wait_sec: f32,
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

#[derive(Debug, Default, Clone)]
pub struct NpcLifeBook {
    pub food_eaten: std::collections::HashMap<i32, u32>,
    pub deaths: Vec<(i32, f32, String)>,
    pub objects_created: std::collections::HashMap<String, u32>,
}

pub struct NpcActivityLog {
    events: Mutex<VecDeque<NpcActivityEvent>>,
    life: Mutex<NpcLifeBook>,
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
            life: Mutex::new(NpcLifeBook::default()),
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
                let key = ev.detail.split_whitespace().next().unwrap_or("craft").to_string();
                if let Ok(mut life) = self.life.lock() {
                    *life.objects_created.entry(key).or_insert(0) += 1;
                }
            }
            NpcActivityKind::Eat | NpcActivityKind::SeekFood => {
                self.eat_attempts.fetch_add(1, Relaxed);
                self.game_ms_eat.fetch_add(ev.game_ms as u64, Relaxed);
                if ev.held_id > 0 {
                    if let Ok(mut life) = self.life.lock() {
                        *life.food_eaten.entry(ev.held_id).or_insert(0) += 1;
                    }
                }
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
                if let Ok(mut life) = self.life.lock() {
                    life.deaths.push((ev.p_id, ev.age, ev.detail.clone()));
                    if life.deaths.len() > 80 {
                        let n = life.deaths.len() - 80;
                        life.deaths.drain(0..n);
                    }
                }
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
        let md_path = self
            .path
            .parent()
            .map(|p| p.join("ai_life_stats.md"))
            .unwrap_or_else(|| PathBuf::from("ai_life_stats.md"));
        let _ = std::fs::write(&md_path, self.format_life_markdown());
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
        let md = self.format_life_markdown();
        let life = self.life.lock().ok();
        let food: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(l
                    .food_eaten
                    .iter()
                    .map(|(id, n)| serde_json::json!({"id": id, "count": n}))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let deaths: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(l
                    .deaths
                    .iter()
                    .map(|(pid, age, why)| serde_json::json!({
                        "p_id": pid, "age": age, "reason": why
                    }))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let objects: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(l
                    .objects_created
                    .iter()
                    .map(|(k, n)| serde_json::json!({"action": k, "count": n}))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
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
            "food_eaten": food,
            "death_log": deaths,
            "objects_created": objects,
            "life_markdown": md,
        })
    }

    pub fn format_life_markdown(&self) -> String {
        use std::sync::atomic::Ordering::*;
        let life = self.life.lock().ok();
        let mut md = String::from("# AI life stats\n\n");
        md.push_str(&format!(
            "- Eat attempts: {}\n- Craft attempts: {}\n- Deaths: {}\n- Stuck events: {}\n\n",
            self.eat_attempts.load(Relaxed),
            self.craft_attempts.load(Relaxed),
            self.deaths.load(Relaxed),
            self.stuck_events.load(Relaxed),
        ));
        md.push_str("## Food eaten (held id → count)\n\n");
        if let Some(l) = life.as_ref() {
            if l.food_eaten.is_empty() {
                md.push_str("_none yet_\n\n");
            } else {
                let mut rows: Vec<_> = l.food_eaten.iter().collect();
                rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                for (id, n) in rows {
                    md.push_str(&format!("- object `{id}` × {n}\n"));
                }
                md.push('\n');
            }
            md.push_str("## Deaths (age, reason)\n\n");
            if l.deaths.is_empty() {
                md.push_str("_none yet_\n\n");
            } else {
                for (pid, age, why) in &l.deaths {
                    md.push_str(&format!("- p_id {pid} age {age:.1}: {why}\n"));
                }
                md.push('\n');
            }
            md.push_str("## Objects / actions created\n\n");
            if l.objects_created.is_empty() {
                md.push_str("_none yet_\n");
            } else {
                let mut rows: Vec<_> = l.objects_created.iter().collect();
                rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                for (k, n) in rows {
                    md.push_str(&format!("- `{k}` × {n}\n"));
                }
            }
        }
        md
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

    #[test]
    fn life_markdown_lists_food_death_and_craft() {
        let log = NpcActivityLog::new(
            std::env::temp_dir().join("ol_npc_life_test.journal"),
            64,
            30,
        );
        log.push(NpcActivityEvent {
            wall_unix_ms: 1,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Eat,
            cpu_us: 10,
            game_ms: 100,
            age: 20.0,
            food: 4.0,
            x: 0,
            y: 0,
            held_id: 30,
            detail: "eat berry".into(),
        });
        log.push(NpcActivityEvent {
            wall_unix_ms: 2,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Craft,
            cpu_us: 10,
            game_ms: 100,
            age: 21.0,
            food: 8.0,
            x: 1,
            y: 1,
            held_id: 0,
            detail: "prof_goc_use actor=33".into(),
        });
        log.push(NpcActivityEvent {
            wall_unix_ms: 3,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Death,
            cpu_us: 1,
            game_ms: 0,
            age: 58.2,
            food: 0.0,
            x: 2,
            y: 2,
            held_id: 0,
            detail: "reason=deleted_or_starved".into(),
        });
        let md = log.format_life_markdown();
        assert!(md.contains("object `30`"));
        assert!(md.contains("age 58.2"));
        assert!(md.contains("prof_goc_use"));
        let j = log.summary_json();
        assert_eq!(j["eat_attempts"], 1);
        assert_eq!(j["deaths"], 1);
    }
}

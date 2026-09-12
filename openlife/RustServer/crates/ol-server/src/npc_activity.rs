//! In-memory NPC activity log with periodic disk flush (default 30s).
//!
//! Tracks what NPCs attempt (craft/eat/move/combat), time spent, stuck loops,
//! death age/reason. No per-event disk I/O.

use std::collections::{HashMap, HashSet, VecDeque};
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
    Baby,
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
            Self::Baby => "baby",
            Self::Stuck => "stuck",
            Self::StuckCycle => "stuck_cycle",
            Self::Death => "death",
            Self::Error => "error",
        }
    }
}

/// Death-age histogram labels (Haxe years). Last bucket is 60+.
pub const DEATH_AGE_LABELS: [&str; 10] = [
    "0-5", "5-10", "10-15", "15-20", "20-25", "25-30", "30-40", "40-50", "50-60", "60+",
];

/// Stable keys for the AI-obs time-spent graphs (not CPU timings).
pub const SPEND_KEYS: [&str; 20] = [
    "walk_target",
    "walk_use",
    "walk_baby",
    "walk_drop",
    "walk_food",
    "walk_follow",
    "walk_home",
    "escape_animal",
    "escape_player",
    "attack_player",
    "eat",
    "craft_use",
    "nurse",
    "pickup_baby",
    "drop_baby",
    "name_baby",
    "wait_baby",
    "explore",
    "stuck",
    "think",
];

pub fn spend_label(key: &str) -> &'static str {
    match key {
        "walk_target" => "Walking to target",
        "walk_use" => "Walking to use item",
        "walk_baby" => "Walking to baby",
        "walk_drop" => "Walking to drop",
        "walk_food" => "Walking to food",
        "walk_follow" => "Following",
        "walk_home" => "Walking home",
        "escape_animal" => "Escaping animals",
        "escape_player" => "Escaping hostile players",
        "attack_player" => "Attacking hostile players",
        "eat" => "Eating",
        "craft_use" => "Using / crafting",
        "nurse" => "Nursing",
        "pickup_baby" => "Picking up baby",
        "drop_baby" => "Dropping baby",
        "name_baby" => "Naming baby",
        "wait_baby" => "Baby waiting on mother",
        "explore" => "Exploring",
        "stuck" => "Stuck",
        "think" => "Thinking / other",
        _ => "Other",
    }
}

pub fn death_age_bucket(age: f32) -> usize {
    if !age.is_finite() || age < 5.0 {
        0
    } else if age < 10.0 {
        1
    } else if age < 15.0 {
        2
    } else if age < 20.0 {
        3
    } else if age < 25.0 {
        4
    } else if age < 30.0 {
        5
    } else if age < 40.0 {
        6
    } else if age < 50.0 {
        7
    } else if age < 60.0 {
        8
    } else {
        9
    }
}

fn parse_i32_after(detail: &str, needle: &str) -> Option<i32> {
    let i = detail.find(needle)?;
    let rest = &detail[i + needle.len()..];
    let mut n = 0i32;
    let mut any = false;
    for c in rest.chars() {
        if let Some(d) = c.to_digit(10) {
            any = true;
            n = n.saturating_mul(10).saturating_add(d as i32);
        } else if any {
            break;
        } else if c == '-' {
            continue;
        } else {
            break;
        }
    }
    if any && n > 0 {
        Some(n)
    } else {
        None
    }
}

pub fn parse_child_p_id(detail: &str) -> Option<i32> {
    parse_i32_after(detail, "child=")
}

/// Food object id for AI-obs ate table: event held_id, else `eat_held=` / `held=` in detail.
pub fn parse_eat_object_id(held_id: i32, detail: &str) -> i32 {
    if held_id > 0 {
        return held_id;
    }
    if let Some(n) = parse_i32_after(detail, "eat_held=") {
        return n;
    }
    if detail.contains("eat_refuse") {
        return 0;
    }
    parse_i32_after(detail, "held=").unwrap_or(0)
}

pub fn parse_crafted_object_ids(detail: &str) -> Vec<i32> {
    let mut v = Vec::new();
    if let Some(n) = parse_i32_after(detail, "CraftItem(") {
        v.push(n);
    }
    if let Some(n) = parse_i32_after(detail, "make_sharpie_food ") {
        v.push(n);
    }
    if let Some(n) = parse_i32_after(detail, "prod=") {
        v.push(n);
    }
    if let Some(n) = parse_i32_after(detail, "->") {
        v.push(n);
    }
    if let Some(n) = parse_i32_after(detail, "→") {
        v.push(n);
    }
    let use_like = detail.contains("_use")
        || detail.contains("arrive")
        || detail.contains("CraftItem")
        || detail.contains("make_sharpie");
    if use_like {
        if let Some(n) = parse_i32_after(detail, "actor=") {
            v.push(n);
        }
        if let Some(n) = parse_i32_after(detail, "target=") {
            v.push(n);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

pub fn classify_spend(kind: NpcActivityKind, detail: &str) -> &'static str {
    let d = detail.to_ascii_lowercase();
    if d.contains("escape_animal") {
        return "escape_animal";
    }
    if d.contains("escape_player") {
        return "escape_player";
    }
    if d.contains("attack_kill") || d.starts_with("hit ") || d.contains("attack_") {
        return "attack_player";
    }
    if d.contains("you_are") {
        return "name_baby";
    }
    if d.contains("pickup child") || d.starts_with("pickup ") {
        return "pickup_baby";
    }
    if d.contains("drop_full")
        || d.contains("drop_cannot_feed")
        || d.contains("drop_obj_for_baby")
        || d.contains("food_drop_held_player")
        || d.contains("drop_baby")
    {
        return "drop_baby";
    }
    if d.contains("hold_nurse") {
        return "nurse";
    }
    if d.contains("baby_wait") {
        return "wait_baby";
    }
    if d.contains("goto_feed") || d.contains("baby_follow") || d.contains("interrupt_walk_for_baby")
    {
        return "walk_baby";
    }
    if d.contains("follow_walk") || d.contains("follow_busy") {
        return "walk_follow";
    }
    if d.contains("handle_death_home") {
        return "walk_home";
    }
    if d.contains("craft_queue_walk_use")
        || d.contains("use_held_walk")
        || d.contains("prof_goc_walk_use")
        || d.contains("walk_use")
    {
        return "walk_use";
    }
    if d.contains("craft_queue_drop")
        || d.contains("smart_drop_walk")
        || d.contains("walk_drop")
        || d.contains("grave_drop")
    {
        return "walk_drop";
    }
    if d.contains("walk_food") || d.contains("pickup_busy") || d.contains("food_goto") {
        return "walk_food";
    }
    if d.contains("craft_queue_walk")
        || d.contains("walk_craft")
        || d.contains("walk_target")
        || d.contains("prof_goc_walk")
        || d.contains("grave_walk")
        || d.contains("grave_seek")
    {
        return "walk_target";
    }
    if d.contains("use_held_arrive")
        || d.contains("craft_queue_use")
        || d.contains("prof_goc_use")
        || d.contains("make_sharpie")
        || d.contains("clothing_self")
    {
        return "craft_use";
    }
    if d.contains("eat_held") || d.contains("eat_refuse") {
        return "eat";
    }
    if d.contains("explore") {
        return "explore";
    }
    match kind {
        NpcActivityKind::SeekFood => "walk_food",
        NpcActivityKind::Eat => "eat",
        NpcActivityKind::Combat => "attack_player",
        NpcActivityKind::Baby | NpcActivityKind::Feed => "nurse",
        NpcActivityKind::Craft | NpcActivityKind::CraftPlan => "craft_use",
        NpcActivityKind::Explore => "explore",
        NpcActivityKind::Stuck | NpcActivityKind::StuckCycle => "stuck",
        NpcActivityKind::Move => "walk_target",
        _ => "think",
    }
}

fn is_baby_named_detail(detail: &str) -> bool {
    detail.to_ascii_lowercase().contains("you_are")
}

fn is_baby_pickup_detail(detail: &str) -> bool {
    let d = detail.to_ascii_lowercase();
    d.contains("pickup child") || d.starts_with("pickup ")
}

fn is_baby_drop_detail(detail: &str) -> bool {
    let d = detail.to_ascii_lowercase();
    d.contains("drop_full")
        || d.contains("drop_cannot_feed")
        || d.contains("drop_obj_for_baby")
        || d.contains("food_drop_held_player")
        || d.contains("drop_baby")
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
    /// True once this slot has had a living view. Missing view before that is
    /// a pending first LOGIN, not a death (player_views lag the intent).
    pub ever_alive: bool,
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
pub struct NpcObsSample {
    pub wall_unix_ms: u64,
    pub named: u64,
    pub named_unique: u64,
    pub pickup: u64,
    pub pickup_unique: u64,
    pub drop_baby: u64,
    pub deaths: u64,
    pub eat_attempts: u64,
    pub craft_attempts: u64,
    pub stuck: u64,
    pub death_ages: [u32; 10],
    pub spend_ms: HashMap<String, u64>,
    pub crafted: Vec<(i32, u32)>,
    pub actions: Vec<(String, u32)>,
}

#[derive(Debug, Default, Clone)]
pub struct NpcLifeBook {
    pub food_eaten: HashMap<i32, u32>,
    pub deaths: Vec<(i32, f32, String)>,
    pub objects_created: HashMap<String, u32>,
    pub crafted_objects: HashMap<i32, u32>,
    pub baby_named: u64,
    pub baby_pickup: u64,
    pub baby_drop: u64,
    pub named_ids: HashSet<i32>,
    pub pickup_ids: HashSet<i32>,
    pub death_ages: [u32; 10],
    pub spend_ms: HashMap<String, u64>,
    pub samples: Vec<NpcObsSample>,
}

const MS_HOUR: u64 = 3_600_000;
const MS_DAY: u64 = 86_400_000;
const MS_WEEK: u64 = 7 * MS_DAY;
const MS_MONTH: u64 = 30 * MS_DAY;
const NPC_OBS_SERIES_MAX: usize = 4096;

fn compact_obs_samples(samples: &mut Vec<NpcObsSample>, now_ms: u64) {
    if samples.len() < 3 {
        return;
    }
    let mut keep: Vec<NpcObsSample> = Vec::with_capacity(samples.len());
    for s in samples.drain(..) {
        let age = now_ms.saturating_sub(s.wall_unix_ms);
        let slot = s.wall_unix_ms / 5_000;
        let keep_it = if age <= MS_HOUR {
            true
        } else if age <= MS_DAY {
            slot % 12 == 0
        } else if age <= MS_WEEK {
            slot % 180 == 0
        } else if age <= MS_MONTH {
            slot % 720 == 0
        } else {
            false
        };
        if keep_it {
            keep.push(s);
        }
    }
    if keep.len() > NPC_OBS_SERIES_MAX {
        let n = keep.len() - NPC_OBS_SERIES_MAX;
        keep.drain(0..n);
    }
    *samples = keep;
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
            NpcActivityKind::Baby | NpcActivityKind::Feed | NpcActivityKind::Combat => {
                self.game_ms_other
                    .fetch_add(ev.game_ms as u64, Relaxed);
                let prefix = ev.kind.as_str();
                let rest = ev.detail.split_whitespace().next().unwrap_or(prefix);
                let key = format!("{prefix}:{rest}");
                if let Ok(mut life) = self.life.lock() {
                    *life.objects_created.entry(key).or_insert(0) += 1;
                }
            }
            NpcActivityKind::Eat | NpcActivityKind::SeekFood => {
                self.eat_attempts.fetch_add(1, Relaxed);
                self.game_ms_eat.fetch_add(ev.game_ms as u64, Relaxed);
                if ev.kind == NpcActivityKind::Eat {
                    let food_id = parse_eat_object_id(ev.held_id, &ev.detail);
                    if food_id > 0 {
                        if let Ok(mut life) = self.life.lock() {
                            *life.food_eaten.entry(food_id).or_insert(0) += 1;
                        }
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
                    let b = death_age_bucket(ev.age);
                    life.death_ages[b] = life.death_ages[b].saturating_add(1);
                }
            }
            _ => {
                self.game_ms_other
                    .fetch_add(ev.game_ms as u64, Relaxed);
            }
        }
        let spend = classify_spend(ev.kind, &ev.detail);
        if let Ok(mut life) = self.life.lock() {
            *life.spend_ms.entry(spend.to_string()).or_insert(0) += ev.game_ms as u64;
            if is_baby_named_detail(&ev.detail) {
                life.baby_named = life.baby_named.saturating_add(1);
                if let Some(id) = parse_child_p_id(&ev.detail) {
                    life.named_ids.insert(id);
                } else {
                    life.named_ids.insert(ev.p_id);
                }
            }
            if is_baby_pickup_detail(&ev.detail) {
                life.baby_pickup = life.baby_pickup.saturating_add(1);
                if let Some(id) = parse_child_p_id(&ev.detail) {
                    life.pickup_ids.insert(id);
                }
            }
            if is_baby_drop_detail(&ev.detail) {
                life.baby_drop = life.baby_drop.saturating_add(1);
            }
            if ev.kind == NpcActivityKind::Craft {
                for id in parse_crafted_object_ids(&ev.detail) {
                    *life.crafted_objects.entry(id).or_insert(0) += 1;
                }
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

    /// Snapshot current totals for AI-obs graphs (called on the stats refresh).
    pub fn record_obs_sample(&self) {
        use std::sync::atomic::Ordering::*;
        let now = Self::wall_ms();
        let Ok(mut life) = self.life.lock() else {
            return;
        };
        let mut crafted: Vec<(i32, u32)> = life
            .crafted_objects
            .iter()
            .map(|(&id, &n)| (id, n))
            .collect();
        crafted.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        crafted.truncate(20);
        let mut actions: Vec<(String, u32)> = life
            .objects_created
            .iter()
            .map(|(k, n)| (k.clone(), *n))
            .collect();
        actions.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        actions.truncate(20);
        let sample = NpcObsSample {
            wall_unix_ms: now,
            named: life.baby_named,
            named_unique: life.named_ids.len() as u64,
            pickup: life.baby_pickup,
            pickup_unique: life.pickup_ids.len() as u64,
            drop_baby: life.baby_drop,
            deaths: self.deaths.load(Relaxed),
            eat_attempts: self.eat_attempts.load(Relaxed),
            craft_attempts: self.craft_attempts.load(Relaxed),
            stuck: self.stuck_events.load(Relaxed),
            death_ages: life.death_ages,
            spend_ms: life.spend_ms.clone(),
            crafted,
            actions,
        };
        life.samples.push(sample);
        compact_obs_samples(&mut life.samples, now);
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
        let crafted: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(l
                    .crafted_objects
                    .iter()
                    .map(|(id, n)| serde_json::json!({"id": id, "count": n}))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let death_ages: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(DEATH_AGE_LABELS
                    .iter()
                    .enumerate()
                    .map(|(i, lab)| serde_json::json!({"bucket": lab, "count": l.death_ages[i]}))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let spend: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(SPEND_KEYS
                    .iter()
                    .map(|k| serde_json::json!({
                        "key": k,
                        "label": spend_label(k),
                        "ms": l.spend_ms.get(*k).copied().unwrap_or(0),
                    }))
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let samples: serde_json::Value = life
            .as_ref()
            .map(|l| {
                serde_json::json!(l
                    .samples
                    .iter()
                    .map(|s| {
                        let spend_obj: serde_json::Map<String, serde_json::Value> = SPEND_KEYS
                            .iter()
                            .map(|k| {
                                (
                                    (*k).to_string(),
                                    serde_json::json!(s.spend_ms.get(*k).copied().unwrap_or(0)),
                                )
                            })
                            .collect();
                        serde_json::json!({
                            "wall_unix_ms": s.wall_unix_ms,
                            "named": s.named,
                            "named_unique": s.named_unique,
                            "pickup": s.pickup,
                            "pickup_unique": s.pickup_unique,
                            "drop_baby": s.drop_baby,
                            "deaths": s.deaths,
                            "eat_attempts": s.eat_attempts,
                            "craft_attempts": s.craft_attempts,
                            "stuck": s.stuck,
                            "death_ages": s.death_ages,
                            "spend_ms": spend_obj,
                            "crafted": s.crafted.iter().map(|(id, n)| serde_json::json!([id, n])).collect::<Vec<_>>(),
                            "actions": s.actions.iter().map(|(k, n)| serde_json::json!([k, n])).collect::<Vec<_>>(),
                        })
                    })
                    .collect::<Vec<_>>())
            })
            .unwrap_or_else(|| serde_json::json!([]));
        let named = life.as_ref().map(|l| l.baby_named).unwrap_or(0);
        let named_unique = life.as_ref().map(|l| l.named_ids.len() as u64).unwrap_or(0);
        let pickup = life.as_ref().map(|l| l.baby_pickup).unwrap_or(0);
        let pickup_unique = life.as_ref().map(|l| l.pickup_ids.len() as u64).unwrap_or(0);
        let drop_baby = life.as_ref().map(|l| l.baby_drop).unwrap_or(0);
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
            "crafted_objects": crafted,
            "death_ages": death_ages,
            "spend_ms": spend,
            "baby_named": named,
            "baby_named_unique": named_unique,
            "baby_pickup": pickup,
            "baby_pickup_unique": pickup_unique,
            "baby_drop": drop_baby,
            "samples": samples,
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

    #[test]
    fn classify_spend_haxe_like_details() {
        assert_eq!(
            classify_spend(NpcActivityKind::Combat, "escape_Animal @1,2"),
            "escape_animal"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Combat, "escape_Player @1,2"),
            "escape_player"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Combat, "attack_kill target=9 @1,2"),
            "attack_player"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Baby, "you_are child=12 YOU ARE ALICE"),
            "name_baby"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Baby, "pickup child=12 @3,4"),
            "pickup_baby"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Baby, "drop_full child=12 food=3.9"),
            "drop_baby"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Craft, "craft_queue_walk_use @1,2"),
            "walk_use"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Craft, "craft_queue_walk @1,2"),
            "walk_target"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::SeekFood, "walk_food id=2143 @1,2"),
            "walk_food"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Move, "walk_target moving"),
            "walk_target"
        );
        assert_eq!(
            classify_spend(NpcActivityKind::Move, "walk_use use_held @1,2"),
            "walk_use"
        );
    }

    #[test]
    fn death_age_buckets_span_child_to_elder() {
        assert_eq!(death_age_bucket(0.4), 0);
        assert_eq!(death_age_bucket(7.0), 1);
        assert_eq!(death_age_bucket(15.2), 3);
        assert_eq!(death_age_bucket(62.0), 9);
    }

    #[test]
    fn parse_crafted_and_baby_ids() {
        assert_eq!(
            parse_crafted_object_ids("clothing_craft CraftItem(128)"),
            vec![128]
        );
        assert_eq!(
            parse_crafted_object_ids("prof_goc_use actor=33 target=36 @1,2"),
            vec![33, 36]
        );
        assert_eq!(
            parse_crafted_object_ids("use craft 0+30->31/30 score=5.2"),
            vec![31]
        );
        assert_eq!(
            parse_crafted_object_ids("plan 0+30 score=5.2 time=0.5s prod=31/30 in=0.0"),
            vec![31]
        );
        assert_eq!(parse_child_p_id("pickup child=9100003 @1,2"), Some(9100003));
    }

    #[test]
    fn summary_tracks_babies_ages_and_samples() {
        let log = NpcActivityLog::new(
            std::env::temp_dir().join("ol_npc_obs_test.journal"),
            64,
            30,
        );
        log.push(NpcActivityEvent {
            wall_unix_ms: 1,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Baby,
            cpu_us: 10,
            game_ms: 500,
            age: 20.0,
            food: 8.0,
            x: 0,
            y: 0,
            held_id: 0,
            detail: "pickup child=42 @1,2".into(),
        });
        log.push(NpcActivityEvent {
            wall_unix_ms: 2,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Baby,
            cpu_us: 10,
            game_ms: 400,
            age: 20.0,
            food: 8.0,
            x: 0,
            y: 0,
            held_id: 0,
            detail: "you_are child=42 YOU ARE ALICE".into(),
        });
        log.push(NpcActivityEvent {
            wall_unix_ms: 3,
            conn_id: 9_100_001,
            p_id: 2,
            kind: NpcActivityKind::Death,
            cpu_us: 1,
            game_ms: 0,
            age: 0.5,
            food: -1.0,
            x: 0,
            y: 0,
            held_id: 0,
            detail: "age=0.5 starved".into(),
        });
        log.record_obs_sample();
        let j = log.summary_json();
        assert_eq!(j["baby_pickup"], 1);
        assert_eq!(j["baby_named"], 1);
        assert_eq!(j["baby_named_unique"], 1);
        assert_eq!(j["deaths"], 1);
        log.push(NpcActivityEvent {
            wall_unix_ms: 4,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Craft,
            cpu_us: 10,
            game_ms: 250,
            age: 20.0,
            food: 8.0,
            x: 0,
            y: 0,
            held_id: 0,
            detail: "use craft 0+30->31/30 score=5.2".into(),
        });
        log.record_obs_sample();
        let j = log.summary_json();
        let crafted = j["crafted_objects"].as_array().expect("crafted");
        assert!(
            crafted.iter().any(|r| r["id"] == 31 && r["count"] == 1),
            "crafted_objects should record gooseberry 31 from use-craft product, got {crafted:?}"
        );
        log.push(NpcActivityEvent {
            wall_unix_ms: 5,
            conn_id: 9_100_000,
            p_id: 1,
            kind: NpcActivityKind::Eat,
            cpu_us: 10,
            game_ms: 100,
            age: 20.0,
            food: 4.0,
            x: 0,
            y: 0,
            held_id: 0,
            detail: "eat_held=31".into(),
        });
        let j = log.summary_json();
        let food = j["food_eaten"].as_array().expect("food");
        assert!(
            food.iter().any(|r| r["id"] == 31),
            "food_eaten must include eat_held= id when held_id is 0, got {food:?}"
        );
        assert_eq!(parse_eat_object_id(0, "eat_held=2143"), 2143);
        assert_eq!(parse_eat_object_id(30, "eat berry"), 30);
        let ages = j["death_ages"].as_array().expect("ages");
        assert_eq!(ages[0]["count"], 1);
        assert!(j["samples"].as_array().unwrap().len() >= 1);
        let spend = j["spend_ms"].as_array().expect("spend");
        assert!(spend.iter().any(|r| r["key"] == "pickup_baby" && r["ms"] == 500));
    }
}

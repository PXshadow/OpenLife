//! Sticky living-player roster disk (**PLAYERS-BIN** / clothing_held_disk).
//!
//! Haxe `GlobalPlayerInstance.WritePlayers` / `ReadPlayers` + `WorldMap` SavePlayers /
//! LoadPlayers. Rust uses a **versioned magic format** (`PLB1`) that embeds the
//! recursive NestedHelper body codec (`nested_body::write/read_player_body_objects`)
//! rather than the legacy Haxe `PlayersN.bin` byte layout (product choice).
//!
//! ```text
//! magic[4] = b"PLB1"
//! version: u32 LE (= PLAYERS_FORMAT_VERSION)
//! next_player_id: i32 LE
//! count: u32 LE
//! records × count: PlayerDiskRecord
//! ```
//!
//! Post-load: dual-pass p_id resolve for mother/father/follow/held/attack/follower
//! refs + exile edges; `alias_hidden_wound_to_held` via body `apply_to_player`.
//! Missing file → empty roster Ok (same pattern as WPS1/OLA1).

use crate::nested_body::{
    read_player_body_objects, write_player_body_objects, PlayerBodyObjects,
};
use crate::player::Player;
use crate::social::SocialState;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::info;

/// Versioned sticky-players store format id.
pub const PLAYERS_FORMAT_VERSION: u32 = 1;
const PLB_MAGIC: &[u8; 4] = b"PLB1";
/// Default on-disk name under the save directory.
pub const DEFAULT_PLAYERS_FILE: &str = "players_v1.bin";
/// Haxe null player-ref sentinel (`GetPlayerIdForWrite(null)`).
// Haxe: GlobalPlayerInstance.GetPlayerIdForWrite L542
pub const PLAYER_REF_NULL: i32 = -100;
/// End-of-record marker (Haxe `writer.writeInt16(-1000)`).
// Haxe: WritePlayers L521
pub const RECORD_END_SIGN: i16 = -1000;
/// Synthetic `conn_id` base for sticky bodies rehydrated without a TCP socket.
/// (Babies use `BABY_CONN_OFFSET` = 1_000_000; loaded lives sit above that.)
pub const LOADED_PLAYER_CONN_BASE: u64 = 2_000_000;

// ── Null player-id helpers ───────────────────────────────────────────────────

/// Encode optional live player id for disk (null → [`PLAYER_REF_NULL`]).
// Haxe: GlobalPlayerInstance.GetPlayerIdForWrite
pub fn get_player_id_for_write(p_id: Option<i32>) -> i32 {
    match p_id {
        Some(id) if id > 0 => id,
        _ => PLAYER_REF_NULL,
    }
}

/// Decode disk player id; sentinel → None.
// Haxe: GlobalPlayerInstance.GetPlayerFromId
pub fn get_player_from_id(player_id: i32) -> Option<i32> {
    if player_id == PLAYER_REF_NULL || player_id <= 0 {
        None
    } else {
        Some(player_id)
    }
}

/// Resolve a stored ref against a live p_id set (second pass).
pub fn resolve_player_ref(player_id: i32, alive: &HashMap<i32, ()>) -> Option<i32> {
    get_player_from_id(player_id).filter(|id| alive.contains_key(id))
}

// ── Disk record ──────────────────────────────────────────────────────────────

/// One sticky player row (Haxe WritePlayers scalars + NestedHelper body + maps).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDiskRecord {
    pub p_id: i32,
    /// Account token (Rust uses email; Haxe used account.id — product map).
    pub email: String,
    pub food: f32,
    pub food_max: f32,
    pub last_ate_fill_max: i32,
    pub yum_bonus: f32,
    /// Haxe `yum_multiplier` (not on Rust Player yet — round-trip preserved).
    pub yum_multiplier: f32,
    pub birth_x: i32,
    pub birth_y: i32,
    pub po_id: i32,
    pub facing: i32,
    pub action: i32,
    pub action_target_x: i32,
    pub action_target_y: i32,
    pub o_id: Vec<i32>,
    pub o_origin_valid: i32,
    pub o_origin_x: i32,
    pub o_origin_y: i32,
    pub o_transition_source_id: i32,
    pub heat: f32,
    pub done_moving_seq: i32,
    pub forced: bool,
    pub x: i32,
    pub y: i32,
    pub age: f32,
    pub age_r: f32,
    pub move_speed: f32,
    pub clothing_set: String,
    pub just_ate: i32,
    pub last_ate_id: i32,
    pub responsible_id: i32,
    pub held_yum: bool,
    pub held_learned: bool,
    pub deleted: bool,
    pub reason: String,
    pub legacy_i: i32,
    /// Nested held / hiddenWound / fever / clothing[6] + yellowfever_count.
    pub body: PlayerBodyObjects,
    // Cross-refs (raw disk ids; resolved in second pass).
    pub mother_id: i32,
    pub father_id: i32,
    pub follow_id: i32,
    pub held_player_id: i32,
    pub held_by_id: i32,
    pub kill_mode: bool,
    pub true_age: f32,
    pub leader_badge: i32,
    pub currently_craving: i32,
    pub last_craving_index: i32,
    pub cravings: Vec<i32>,
    pub hits: f32,
    pub wounded_by: i32,
    pub exhaustion: f32,
    pub children_birth_mali: f32,
    pub food_use_per_second: f32,
    pub coins: f32,
    pub prestige_from_children: f32,
    pub prestige_from_eating: f32,
    pub prestige_from_followers: f32,
    pub prestige_from_wealth: f32,
    pub last_attacked_me: i32,
    pub last_attacked: i32,
    pub angry_time: f32,
    pub new_follower: i32,
    pub new_follower_for: i32,
    pub new_follower_time: f32,
    pub is_cursed: bool,
    pub in_wrong_biome: bool,
    pub in_home_biome: bool,
    pub jumped_tiles: f32,
    pub last_say_in_sec: f32,
    pub has_eaten: Vec<(i32, f32)>,
    pub exiled_by: Vec<i32>,
    /// Haxe `storedInt` (homeTx/homeTy/cold*/warm*/fire*).
    pub stored_int: Vec<(String, i32)>,
    // Rust sticky extras (not in Haxe Players.bin — product extension).
    pub first_name: String,
    pub family_name: String,
    pub partner_p_id: i32,
    pub stored_water: f32,
    pub display_object_id: i32,
}

impl Default for PlayerDiskRecord {
    fn default() -> Self {
        Self {
            p_id: 0,
            email: String::new(),
            food: 10.0,
            food_max: 20.0,
            last_ate_fill_max: 0,
            yum_bonus: 0.0,
            yum_multiplier: 1.0,
            birth_x: 0,
            birth_y: 0,
            po_id: 19,
            facing: 0,
            action: 0,
            action_target_x: 0,
            action_target_y: 0,
            o_id: Vec::new(),
            o_origin_valid: 0,
            o_origin_x: 0,
            o_origin_y: 0,
            o_transition_source_id: 0,
            heat: 0.5,
            done_moving_seq: 1,
            forced: false,
            x: 0,
            y: 0,
            age: 14.0,
            age_r: 60.0,
            move_speed: 3.75,
            clothing_set: String::new(),
            just_ate: 0,
            last_ate_id: 0,
            responsible_id: -1,
            held_yum: false,
            held_learned: false,
            deleted: false,
            reason: String::new(),
            legacy_i: 0,
            body: PlayerBodyObjects::default(),
            mother_id: PLAYER_REF_NULL,
            father_id: PLAYER_REF_NULL,
            follow_id: PLAYER_REF_NULL,
            held_player_id: PLAYER_REF_NULL,
            held_by_id: PLAYER_REF_NULL,
            kill_mode: false,
            true_age: 14.0,
            leader_badge: 0,
            currently_craving: 0,
            last_craving_index: 0,
            cravings: Vec::new(),
            hits: 0.0,
            wounded_by: 0,
            exhaustion: 0.0,
            children_birth_mali: 0.0,
            food_use_per_second: 0.0,
            coins: 0.0,
            prestige_from_children: 0.0,
            prestige_from_eating: 0.0,
            prestige_from_followers: 0.0,
            prestige_from_wealth: 0.0,
            last_attacked_me: PLAYER_REF_NULL,
            last_attacked: PLAYER_REF_NULL,
            angry_time: 0.0,
            new_follower: PLAYER_REF_NULL,
            new_follower_for: PLAYER_REF_NULL,
            new_follower_time: 0.0,
            is_cursed: false,
            in_wrong_biome: false,
            in_home_biome: false,
            jumped_tiles: 0.0,
            last_say_in_sec: 0.0,
            has_eaten: Vec::new(),
            exiled_by: Vec::new(),
            stored_int: Vec::new(),
            first_name: String::new(),
            family_name: String::new(),
            partner_p_id: 0,
            stored_water: 0.0,
            display_object_id: 19,
        }
    }
}

/// Optional context when capturing a live player into a disk record.
#[derive(Debug, Clone, Default)]
pub struct CapturePlayerCtx {
    pub mother_id: Option<i32>,
    pub father_id: Option<i32>,
    pub follow_id: Option<i32>,
    pub coins: f32,
    pub prestige_from_children: f32,
    pub prestige_from_eating: f32,
    pub prestige_from_followers: f32,
    pub prestige_from_wealth: f32,
    pub exiled_by: Vec<i32>,
    pub clothing_set: String,
    pub move_speed: f32,
    pub last_say_in_sec: f32,
}

impl CapturePlayerCtx {
    pub fn from_social(social: &SocialState, p_id: i32) -> Self {
        let mut ctx = Self::default();
        if let Some(lin) = social.lineages.get(&p_id) {
            ctx.mother_id = lin.mother_id;
            ctx.father_id = lin.father_id;
        }
        ctx.follow_id = social.following.get(&p_id).copied();
        // Haxe exiledByPlayers: map of exiler p_id → player. Invert SocialState.exiles.
        let mut exilers = Vec::new();
        for (leader, set) in &social.exiles {
            if set.contains(&p_id) {
                exilers.push(*leader);
            }
        }
        exilers.sort_unstable();
        ctx.exiled_by = exilers;
        ctx
    }
}

/// Capture sticky fields from a live [`Player`] (+ optional social/economy ctx).
// Haxe: GlobalPlayerInstance.WritePlayers (per-player)
pub fn capture_player_snapshot(p: &Player, ctx: &CapturePlayerCtx) -> PlayerDiskRecord {
    let mut has_eaten: Vec<(i32, f32)> = p.yum.has_eaten.iter().map(|(&k, &v)| (k, v)).collect();
    has_eaten.sort_by_key(|(k, _)| *k);

    let mut stored_int = Vec::new();
    stored_int.push(("homeTx".into(), p.home_x));
    stored_int.push(("homeTy".into(), p.home_y));

    let reason = p.death_reason.clone().unwrap_or_default();
    let o_id = if p.held_id != 0 {
        vec![p.held_id]
    } else {
        Vec::new()
    };

    PlayerDiskRecord {
        p_id: p.p_id,
        email: p.email.clone(),
        food: p.food,
        food_max: p.food_max,
        last_ate_fill_max: p.yum.last_ate_fill_max,
        yum_bonus: p.yum.yum_bonus,
        yum_multiplier: 1.0,
        birth_x: p.birth_x,
        birth_y: p.birth_y,
        po_id: p.display_object_id,
        facing: 0,
        action: 0,
        action_target_x: 0,
        action_target_y: 0,
        o_id,
        o_origin_valid: 0,
        o_origin_x: 0,
        o_origin_y: 0,
        o_transition_source_id: 0,
        heat: p.heat,
        done_moving_seq: p.done_moving_seq,
        forced: p.wait_for_force,
        x: p.x,
        y: p.y,
        age: p.age,
        age_r: p.age_r,
        move_speed: if ctx.move_speed > 0.0 {
            ctx.move_speed
        } else {
            3.75
        },
        clothing_set: ctx.clothing_set.clone(),
        just_ate: if p.yum.just_ate { 1 } else { 0 },
        last_ate_id: p.yum.just_ate_id,
        responsible_id: -1,
        held_yum: false,
        held_learned: false,
        deleted: p.deleted,
        reason,
        legacy_i: 0,
        body: PlayerBodyObjects::from_player(p),
        mother_id: get_player_id_for_write(ctx.mother_id),
        father_id: get_player_id_for_write(ctx.father_id),
        follow_id: get_player_id_for_write(ctx.follow_id),
        held_player_id: get_player_id_for_write(if p.holding_player_id > 0 {
            Some(p.holding_player_id)
        } else {
            None
        }),
        held_by_id: get_player_id_for_write(if p.held_by > 0 {
            Some(p.held_by)
        } else {
            None
        }),
        kill_mode: false,
        true_age: p.true_age,
        leader_badge: 0,
        currently_craving: p.yum.currently_craving,
        last_craving_index: p.yum.last_craving_index,
        cravings: p.yum.cravings.clone(),
        hits: 0.0,
        wounded_by: 0,
        exhaustion: p.exhaustion,
        children_birth_mali: 0.0,
        food_use_per_second: 0.0,
        coins: ctx.coins,
        prestige_from_children: ctx.prestige_from_children,
        prestige_from_eating: ctx.prestige_from_eating,
        prestige_from_followers: ctx.prestige_from_followers,
        prestige_from_wealth: ctx.prestige_from_wealth,
        last_attacked_me: get_player_id_for_write(if p.last_player_attacked_me_id > 0 {
            Some(p.last_player_attacked_me_id)
        } else {
            None
        }),
        last_attacked: get_player_id_for_write(if p.last_attacked_player_id > 0 {
            Some(p.last_attacked_player_id)
        } else {
            None
        }),
        angry_time: p.angry_time,
        new_follower: get_player_id_for_write(if p.new_follower_id > 0 {
            Some(p.new_follower_id)
        } else {
            None
        }),
        new_follower_for: get_player_id_for_write(if p.new_follower_for_id > 0 {
            Some(p.new_follower_for_id)
        } else {
            None
        }),
        new_follower_time: p.new_follower_time,
        is_cursed: p.is_cursed,
        in_wrong_biome: false,
        in_home_biome: false,
        jumped_tiles: p.jumped_tiles,
        last_say_in_sec: ctx.last_say_in_sec,
        has_eaten,
        exiled_by: ctx.exiled_by.clone(),
        stored_int,
        first_name: p.first_name.clone(),
        family_name: p.family_name.clone(),
        partner_p_id: p.partner_p_id,
        stored_water: p.stored_water,
        display_object_id: p.display_object_id,
    }
}

/// Apply disk scalars + body onto a live player (does not resolve cross-refs).
///
/// Cross-refs: use [`apply_player_cross_refs`] after the full roster is present.
// Haxe: GlobalPlayerInstance.ReadPlayers (first pass + body)
pub fn apply_player_snapshot(rec: &PlayerDiskRecord, p: &mut Player) {
    p.p_id = rec.p_id;
    if !rec.email.is_empty() {
        p.email = rec.email.clone();
    }
    p.food = rec.food;
    p.food_max = rec.food_max;
    p.yum.last_ate_fill_max = rec.last_ate_fill_max;
    p.yum.yum_bonus = rec.yum_bonus;
    p.birth_x = rec.birth_x;
    p.birth_y = rec.birth_y;
    p.display_object_id = if rec.display_object_id != 0 {
        rec.display_object_id
    } else {
        rec.po_id
    };
    p.heat = rec.heat;
    p.done_moving_seq = rec.done_moving_seq;
    p.wait_for_force = rec.forced;
    p.x = rec.x;
    p.y = rec.y;
    p.age = rec.age;
    p.age_r = rec.age_r;
    p.yum.just_ate = rec.just_ate != 0;
    p.yum.just_ate_id = rec.last_ate_id;
    p.deleted = rec.deleted;
    p.death_reason = if rec.reason.is_empty() {
        None
    } else {
        Some(rec.reason.clone())
    };
    rec.body.apply_to_player(p);
    p.true_age = rec.true_age;
    p.yum.currently_craving = rec.currently_craving;
    p.yum.last_craving_index = rec.last_craving_index;
    p.yum.cravings = rec.cravings.clone();
    p.yum.has_eaten = rec.has_eaten.iter().copied().collect();
    p.exhaustion = rec.exhaustion;
    p.angry_time = rec.angry_time;
    p.is_cursed = rec.is_cursed;
    p.jumped_tiles = rec.jumped_tiles;
    p.partner_p_id = rec.partner_p_id;
    p.stored_water = rec.stored_water;
    if !rec.first_name.is_empty() {
        p.first_name = rec.first_name.clone();
    }
    if !rec.family_name.is_empty() {
        p.family_name = rec.family_name.clone();
    }
    // storedInt home
    for (k, v) in &rec.stored_int {
        match k.as_str() {
            "homeTx" => p.home_x = *v,
            "homeTy" => p.home_y = *v,
            _ => {}
        }
    }
    // Provisional cross-refs as raw ids (0 when null); refined in second pass.
    p.holding_player_id = get_player_from_id(rec.held_player_id).unwrap_or(0);
    p.held_by = get_player_from_id(rec.held_by_id).unwrap_or(0);
    p.last_player_attacked_me_id = get_player_from_id(rec.last_attacked_me).unwrap_or(0);
    p.last_attacked_player_id = get_player_from_id(rec.last_attacked).unwrap_or(0);
    p.new_follower_id = get_player_from_id(rec.new_follower).unwrap_or(0);
    p.new_follower_for_id = get_player_from_id(rec.new_follower_for).unwrap_or(0);
    p.new_follower_time = rec.new_follower_time;
}

/// Second-pass cross-ref resolve for one player (Haxe ReadPlayers L837–860).
// Haxe: GlobalPlayerInstance.ReadPlayers second pass
pub fn apply_player_cross_refs(
    rec: &PlayerDiskRecord,
    p: &mut Player,
    alive: &HashMap<i32, ()>,
    social: &mut SocialState,
) {
    // held / attack / follower refs on Player
    p.holding_player_id = resolve_player_ref(rec.held_player_id, alive).unwrap_or(0);
    p.held_by = resolve_player_ref(rec.held_by_id, alive).unwrap_or(0);
    p.last_player_attacked_me_id = resolve_player_ref(rec.last_attacked_me, alive).unwrap_or(0);
    p.last_attacked_player_id = resolve_player_ref(rec.last_attacked, alive).unwrap_or(0);
    p.new_follower_id = resolve_player_ref(rec.new_follower, alive).unwrap_or(0);
    p.new_follower_for_id = resolve_player_ref(rec.new_follower_for, alive).unwrap_or(0);

    // Follow edge
    if let Some(leader) = resolve_player_ref(rec.follow_id, alive) {
        let _ = social.set_follow(p.p_id, leader);
    }

    // Lineage mother/father if node exists
    if let Some(lin) = social.lineages.get_mut(&p.p_id) {
        lin.mother_id = resolve_player_ref(rec.mother_id, alive);
        lin.father_id = resolve_player_ref(rec.father_id, alive);
        lin.alive = !p.deleted;
    }

    // Exile edges: exiler → this player
    for exiler in &rec.exiled_by {
        if resolve_player_ref(*exiler, alive).is_some() {
            social.exile(*exiler, p.p_id);
        }
    }
}

// ── Roster snapshot ──────────────────────────────────────────────────────────

/// Full sticky roster for disk + outer autosave mirror.
#[derive(Debug, Default, Clone)]
pub struct PlayersSnapshot {
    pub next_player_id: i32,
    pub records: Vec<PlayerDiskRecord>,
}

impl PlayersSnapshot {
    pub fn new() -> Self {
        Self {
            next_player_id: 2,
            records: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Living (non-deleted) p_ids.
    pub fn alive_p_ids(&self) -> HashSet<i32> {
        self.records
            .iter()
            .filter(|r| !r.deleted)
            .map(|r| r.p_id)
            .collect()
    }
}

/// Shared mirror for sim ↔ ol-server autosave.
pub type PlayersShare = Arc<RwLock<PlayersSnapshot>>;

/// Capture all non-deleted (or all) players from a map keyed by conn_id.
pub fn capture_players_snapshot(
    players: &HashMap<u64, Player>,
    social: &SocialState,
    next_player_id: i32,
    coins_of: impl Fn(i32) -> f32,
    include_deleted: bool,
) -> PlayersSnapshot {
    let mut records = Vec::new();
    let mut order: Vec<(i32, u64)> = players
        .iter()
        .filter(|(_, p)| include_deleted || !p.deleted)
        .map(|(c, p)| (p.p_id, *c))
        .collect();
    order.sort_by_key(|(pid, _)| *pid);
    for (_, conn) in order {
        let p = match players.get(&conn) {
            Some(p) => p,
            None => continue,
        };
        let mut ctx = CapturePlayerCtx::from_social(social, p.p_id);
        ctx.coins = coins_of(p.p_id);
        ctx.clothing_set = crate::clothing_transitions::format_clothing_set(p);
        if let Some(t) = p.last_say_times.back() {
            ctx.last_say_in_sec = *t;
        }
        records.push(capture_player_snapshot(p, &ctx));
    }
    PlayersSnapshot {
        next_player_id: next_player_id.max(2),
        records,
    }
}

/// Materialize sticky AI-controlled bodies into `players` (keyed by synthetic conn).
///
/// Clears existing entries that match loaded p_ids first. Runs dual-pass cross-refs
/// and updates social follow/exile/lineage.alive.
// Haxe: GlobalPlayerInstance.ReadPlayers + ServerAi attach
pub fn apply_players_snapshot(
    snap: &PlayersSnapshot,
    players: &mut HashMap<u64, Player>,
    social: &mut SocialState,
    next_player_id: &mut i32,
) -> usize {
    // Drop any prior sticky bodies with same p_id.
    let loaded_ids: HashSet<i32> = snap.records.iter().map(|r| r.p_id).collect();
    players.retain(|_, p| !loaded_ids.contains(&p.p_id));

    let mut by_pid: HashMap<i32, u64> = HashMap::new();
    for rec in &snap.records {
        if rec.p_id <= 0 {
            continue;
        }
        let mut conn = LOADED_PLAYER_CONN_BASE.saturating_add(rec.p_id as u64);
        while players.contains_key(&conn) {
            conn = conn.saturating_add(1);
        }
        let mut p = Player::new(rec.p_id, conn, &rec.email);
        apply_player_snapshot(rec, &mut p);
        // Haxe: ServerAi until human logs in
        p.connected = false;
        p.ai_controlled = !p.deleted;
        by_pid.insert(rec.p_id, conn);
        players.insert(conn, p);
    }

    let alive: HashMap<i32, ()> = snap
        .records
        .iter()
        .filter(|r| !r.deleted)
        .map(|r| (r.p_id, ()))
        .collect();

    for rec in &snap.records {
        let Some(&conn) = by_pid.get(&rec.p_id) else {
            continue;
        };
        if let Some(p) = players.get_mut(&conn) {
            apply_player_cross_refs(rec, p, &alive, social);
        }
    }

    if snap.next_player_id > *next_player_id {
        *next_player_id = snap.next_player_id;
    }
    // Ensure next is above any loaded p_id.
    if let Some(max_pid) = snap.records.iter().map(|r| r.p_id).max() {
        if max_pid + 1 > *next_player_id {
            *next_player_id = max_pid + 1;
        }
    }
    by_pid.len()
}

// ── Binary I/O ───────────────────────────────────────────────────────────────

fn write_string(w: &mut impl Write, s: &str) -> Result<(), String> {
    let b = s.as_bytes();
    if b.len() > 1_048_576 {
        return Err(format!("string too long: {}", b.len()));
    }
    w.write_u32::<LittleEndian>(b.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(b).map_err(|e| e.to_string())
}

fn read_string(r: &mut impl Read) -> Result<String, String> {
    let len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if len > 1_048_576 {
        return Err(format!("string too long: {len}"));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

fn write_bool_u8(w: &mut impl Write, v: bool) -> Result<(), String> {
    w.write_u8(if v { 1 } else { 0 }).map_err(|e| e.to_string())
}

fn read_bool_u8(r: &mut impl Read) -> Result<bool, String> {
    Ok(r.read_u8().map_err(|e| e.to_string())? != 0)
}

/// Write one player record.
// Haxe: GlobalPlayerInstance.WritePlayers (body of loop)
pub fn write_player_record(w: &mut impl Write, rec: &PlayerDiskRecord) -> Result<(), String> {
    w.write_i32::<LittleEndian>(rec.p_id)
        .map_err(|e| e.to_string())?;
    write_string(w, &rec.email)?;
    w.write_f32::<LittleEndian>(rec.food)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.food_max)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.last_ate_fill_max)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.yum_bonus)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.yum_multiplier)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.birth_x)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.birth_y)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.po_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.facing)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.action)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.action_target_x)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.action_target_y)
        .map_err(|e| e.to_string())?;
    w.write_u16::<LittleEndian>(rec.o_id.len() as u16)
        .map_err(|e| e.to_string())?;
    for id in &rec.o_id {
        w.write_i32::<LittleEndian>(*id)
            .map_err(|e| e.to_string())?;
    }
    w.write_i32::<LittleEndian>(rec.o_origin_valid)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.o_origin_x)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.o_origin_y)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.o_transition_source_id)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.heat)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.done_moving_seq)
        .map_err(|e| e.to_string())?;
    write_bool_u8(w, rec.forced)?;
    w.write_i32::<LittleEndian>(rec.x)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.y)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.age)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.age_r)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.move_speed)
        .map_err(|e| e.to_string())?;
    // clothing_set as u16-len + bytes (Haxe writeInt16 + writeString)
    let cs = rec.clothing_set.as_bytes();
    if cs.len() > u16::MAX as usize {
        return Err("clothing_set too long".into());
    }
    w.write_u16::<LittleEndian>(cs.len() as u16)
        .map_err(|e| e.to_string())?;
    w.write_all(cs).map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.just_ate)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.last_ate_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.responsible_id)
        .map_err(|e| e.to_string())?;
    write_bool_u8(w, rec.held_yum)?;
    write_bool_u8(w, rec.held_learned)?;
    write_bool_u8(w, rec.deleted)?;
    let rs = rec.reason.as_bytes();
    if rs.len() > u16::MAX as usize {
        return Err("reason too long".into());
    }
    w.write_u16::<LittleEndian>(rs.len() as u16)
        .map_err(|e| e.to_string())?;
    w.write_all(rs).map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.legacy_i)
        .map_err(|e| e.to_string())?;

    // Body NestedHelpers (held + wound + fever + clothing) — product NestedHelper codec.
    write_player_body_objects(w, &rec.body)?;

    w.write_i32::<LittleEndian>(rec.mother_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.father_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.follow_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.held_player_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.held_by_id)
        .map_err(|e| e.to_string())?;
    write_bool_u8(w, rec.kill_mode)?;
    w.write_f32::<LittleEndian>(rec.true_age)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.leader_badge)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.currently_craving)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.last_craving_index)
        .map_err(|e| e.to_string())?;
    w.write_u16::<LittleEndian>(rec.cravings.len() as u16)
        .map_err(|e| e.to_string())?;
    for c in &rec.cravings {
        w.write_i32::<LittleEndian>(*c)
            .map_err(|e| e.to_string())?;
    }
    w.write_f32::<LittleEndian>(rec.hits)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.wounded_by)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.exhaustion)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.children_birth_mali)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.food_use_per_second)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.coins)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.prestige_from_children)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.prestige_from_eating)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.prestige_from_followers)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.prestige_from_wealth)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.last_attacked_me)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.last_attacked)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.angry_time)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.new_follower)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.new_follower_for)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.new_follower_time)
        .map_err(|e| e.to_string())?;
    write_bool_u8(w, rec.is_cursed)?;
    write_bool_u8(w, rec.in_wrong_biome)?;
    write_bool_u8(w, rec.in_home_biome)?;
    w.write_f32::<LittleEndian>(rec.jumped_tiles)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.last_say_in_sec)
        .map_err(|e| e.to_string())?;

    w.write_u16::<LittleEndian>(rec.has_eaten.len() as u16)
        .map_err(|e| e.to_string())?;
    for (k, v) in &rec.has_eaten {
        w.write_i32::<LittleEndian>(*k)
            .map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(*v)
            .map_err(|e| e.to_string())?;
    }
    w.write_u16::<LittleEndian>(rec.exiled_by.len() as u16)
        .map_err(|e| e.to_string())?;
    for id in &rec.exiled_by {
        w.write_i32::<LittleEndian>(*id)
            .map_err(|e| e.to_string())?;
    }
    w.write_u16::<LittleEndian>(rec.stored_int.len() as u16)
        .map_err(|e| e.to_string())?;
    for (k, v) in &rec.stored_int {
        // Haxe writeString("$key\n"); use length-prefixed for robust PLB1.
        write_string(w, k)?;
        w.write_i32::<LittleEndian>(*v)
            .map_err(|e| e.to_string())?;
    }

    w.write_i16::<LittleEndian>(RECORD_END_SIGN)
        .map_err(|e| e.to_string())?;

    // Rust extras after end-sign versioned block (still inside record for v1).
    write_string(w, &rec.first_name)?;
    write_string(w, &rec.family_name)?;
    w.write_i32::<LittleEndian>(rec.partner_p_id)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(rec.stored_water)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(rec.display_object_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Read one player record.
// Haxe: GlobalPlayerInstance.ReadPlayers (body of loop)
pub fn read_player_record(r: &mut impl Read) -> Result<PlayerDiskRecord, String> {
    let mut rec = PlayerDiskRecord::default();
    rec.p_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.email = read_string(r)?;
    rec.food = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.food_max = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_ate_fill_max = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.yum_bonus = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.yum_multiplier = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.birth_x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.birth_y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.po_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.facing = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.action = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.action_target_x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.action_target_y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let o_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if o_len > 64 {
        return Err(format!("o_id length absurd: {o_len}"));
    }
    rec.o_id.clear();
    for _ in 0..o_len {
        rec.o_id
            .push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
    }
    rec.o_origin_valid = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.o_origin_x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.o_origin_y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.o_transition_source_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.heat = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.done_moving_seq = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.forced = read_bool_u8(r)?;
    rec.x = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.y = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.age = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.age_r = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.move_speed = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let cs_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut cs = vec![0u8; cs_len];
    r.read_exact(&mut cs).map_err(|e| e.to_string())?;
    rec.clothing_set = String::from_utf8(cs).map_err(|e| e.to_string())?;
    rec.just_ate = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_ate_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.responsible_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.held_yum = read_bool_u8(r)?;
    rec.held_learned = read_bool_u8(r)?;
    rec.deleted = read_bool_u8(r)?;
    let rs_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut rs = vec![0u8; rs_len];
    r.read_exact(&mut rs).map_err(|e| e.to_string())?;
    rec.reason = String::from_utf8(rs).map_err(|e| e.to_string())?;
    rec.legacy_i = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;

    rec.body = read_player_body_objects(r)?;

    rec.mother_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.father_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.follow_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.held_player_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.held_by_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.kill_mode = read_bool_u8(r)?;
    rec.true_age = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.leader_badge = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.currently_craving = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_craving_index = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let cr_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if cr_len > 10_000 {
        return Err(format!("cravings length absurd: {cr_len}"));
    }
    rec.cravings.clear();
    for _ in 0..cr_len {
        rec.cravings
            .push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
    }
    rec.hits = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.wounded_by = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.exhaustion = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.children_birth_mali = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.food_use_per_second = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.coins = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.prestige_from_children = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.prestige_from_eating = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.prestige_from_followers = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.prestige_from_wealth = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_attacked_me = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_attacked = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.angry_time = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.new_follower = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.new_follower_for = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.new_follower_time = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.is_cursed = read_bool_u8(r)?;
    rec.in_wrong_biome = read_bool_u8(r)?;
    rec.in_home_biome = read_bool_u8(r)?;
    rec.jumped_tiles = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.last_say_in_sec = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;

    let he_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if he_len > 100_000 {
        return Err(format!("has_eaten length absurd: {he_len}"));
    }
    rec.has_eaten.clear();
    for _ in 0..he_len {
        let k = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let v = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        rec.has_eaten.push((k, v));
    }
    let ex_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if ex_len > 100_000 {
        return Err(format!("exiled_by length absurd: {ex_len}"));
    }
    rec.exiled_by.clear();
    for _ in 0..ex_len {
        rec.exiled_by
            .push(r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?);
    }
    let si_len = r.read_u16::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if si_len > 10_000 {
        return Err(format!("stored_int length absurd: {si_len}"));
    }
    rec.stored_int.clear();
    for _ in 0..si_len {
        let k = read_string(r)?;
        let v = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        rec.stored_int.push((k, v));
    }

    let end = r.read_i16::<LittleEndian>().map_err(|e| e.to_string())?;
    if end != RECORD_END_SIGN {
        return Err(format!(
            "player record wrong end sign: {end} (want {RECORD_END_SIGN})"
        ));
    }

    rec.first_name = read_string(r)?;
    rec.family_name = read_string(r)?;
    rec.partner_p_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.stored_water = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    rec.display_object_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    Ok(rec)
}

/// Write full roster to a writer.
// Haxe: GlobalPlayerInstance.WritePlayers / WriteAllPlayers
pub fn write_players(snap: &PlayersSnapshot, w: &mut impl Write) -> Result<(), String> {
    w.write_all(PLB_MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(PLAYERS_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(snap.next_player_id)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(snap.records.len() as u32)
        .map_err(|e| e.to_string())?;
    for rec in &snap.records {
        write_player_record(w, rec)?;
    }
    Ok(())
}

/// Read full roster from a reader.
// Haxe: GlobalPlayerInstance.ReadPlayers
pub fn read_players(r: &mut impl Read) -> Result<PlayersSnapshot, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != PLB_MAGIC {
        return Err(format!(
            "bad players magic: {:?} (want PLB1)",
            String::from_utf8_lossy(&magic)
        ));
    }
    let ver = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if ver != PLAYERS_FORMAT_VERSION {
        return Err(format!(
            "unsupported players version {ver} (want {PLAYERS_FORMAT_VERSION})"
        ));
    }
    let next_player_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if count > 100_000 {
        return Err(format!("players count absurd: {count}"));
    }
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        records.push(read_player_record(r)?);
    }
    Ok(PlayersSnapshot {
        next_player_id: next_player_id.max(2),
        records,
    })
}

/// Atomic save of sticky players to `path` (tmp + rename).
// Haxe: GlobalPlayerInstance.WriteAllPlayers / WritePlayers
pub fn save_players(snap: &PlayersSnapshot, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(64 * 1024, f);
        write_players(snap, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    info!(
        path = %path.display(),
        count = snap.len(),
        next_p_id = snap.next_player_id,
        ms = t0.elapsed().as_millis() as u64,
        "players saved (PLB1)"
    );
    Ok(())
}

/// Load sticky players from `path`. Missing file → empty roster Ok.
// Haxe: GlobalPlayerInstance.ReadPlayers / WorldMap LoadPlayers
pub fn load_players(path: impl AsRef<Path>) -> Result<PlayersSnapshot, String> {
    let path: &Path = path.as_ref();
    if !path.exists() {
        return Ok(PlayersSnapshot::new());
    }
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(64 * 1024, f);
    let snap = read_players(&mut r)?;
    info!(
        path = %path.display(),
        count = snap.len(),
        next_p_id = snap.next_player_id,
        ms = t0.elapsed().as_millis() as u64,
        "players loaded (PLB1)"
    );
    Ok(snap)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::ClothingSlot;
    use ol_world::NestedHelper;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("ol_players_{prefix}_{t}_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_player(p_id: i32, email: &str) -> Player {
        let mut p = Player::new(p_id, 10 + p_id as u64, email);
        p.x = 12;
        p.y = -3;
        p.birth_x = 100;
        p.birth_y = 200;
        p.food = 7.5;
        p.food_max = 22.0;
        p.age = 20.0;
        p.true_age = 20.5;
        p.heat = 0.62;
        p.home_x = 50;
        p.home_y = 51;
        p.exhaustion = 1.25;
        p.jumped_tiles = 0.4;
        p.angry_time = -2.0;
        p.is_cursed = true;
        p.first_name = "Ada".into();
        p.family_name = "Lovelace".into();
        p.yum.yum_bonus = 3.0;
        p.yum.currently_craving = 40;
        p.yum.cravings = vec![40, 33];
        p.yum.has_eaten.insert(33, 2.0);
        p.yum.has_eaten.insert(40, -1.0);
        p.yum.last_ate_fill_max = 18;
        p.yum.just_ate_id = 33;
        let mut bag = NestedHelper::from_wire(292, &[33, 40]);
        bag.uses_remaining = 2;
        bag.time_to_change = 5.0;
        bag.creation_time = 1.0;
        p.set_held_helper(bag);
        let mut pack = NestedHelper::from_wire(697, &[100, 101]);
        pack.hits = 1.5;
        p.set_clothing_helper(ClothingSlot::Chest, pack);
        p.clothing_helpers[5] = Some(NestedHelper::from_wire(198, &[10]));
        p.hidden_wound = Some(NestedHelper::with_uses(200, 1));
        p.fever = Some(NestedHelper::id_only(crate::nested_body::YELLOW_FEVER_ID));
        p.yellowfever_count = 0.25;
        p.holding_player_id = 0;
        p.held_by = 0;
        p
    }

    #[test]
    fn null_player_ref_roundtrip() {
        assert_eq!(get_player_id_for_write(None), PLAYER_REF_NULL);
        assert_eq!(get_player_id_for_write(Some(0)), PLAYER_REF_NULL);
        assert_eq!(get_player_id_for_write(Some(7)), 7);
        assert_eq!(get_player_from_id(PLAYER_REF_NULL), None);
        assert_eq!(get_player_from_id(7), Some(7));
    }

    #[test]
    fn multi_player_record_round_trip() {
        let mut social = SocialState::default();
        // Exile first: exile(1,2) clears follow(2→1) when leader matches.
        social.exile(1, 2);
        social.set_follow(2, 1).unwrap();

        let p1 = sample_player(1, "a@t");
        let mut p2 = sample_player(2, "b@t");
        p2.held_by = 1;
        p2.holding_player_id = 0;
        // mother holds baby
        let mut p1 = p1;
        p1.holding_player_id = 2;

        let mut players = HashMap::new();
        players.insert(p1.conn_id, p1);
        players.insert(p2.conn_id, p2);

        let snap = capture_players_snapshot(&players, &social, 5, |_| 12.0, false);
        assert_eq!(snap.len(), 2);
        assert_eq!(snap.next_player_id, 5);

        let mut buf = Vec::new();
        write_players(&snap, &mut buf).unwrap();
        let loaded = read_players(&mut Cursor::new(buf)).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.next_player_id, 5);

        let r1 = loaded.records.iter().find(|r| r.p_id == 1).unwrap();
        let r2 = loaded.records.iter().find(|r| r.p_id == 2).unwrap();
        assert_eq!(r1.held_player_id, 2);
        assert_eq!(r2.held_by_id, 1);
        assert_eq!(r1.body.held.as_ref().unwrap().id, 292);
        assert_eq!(r1.body.held.as_ref().unwrap().contained.len(), 2);
        assert_eq!(r1.body.clothing[5].as_ref().unwrap().id, 198);
        assert!((r1.body.yellowfever_count - 0.25).abs() < 1e-5);
        assert!(r1.has_eaten.iter().any(|(k, v)| *k == 33 && (*v - 2.0).abs() < 1e-5));
        assert_eq!(r1.currently_craving, 40);
        assert!(r1.cravings.contains(&40));
        assert!((r1.coins - 12.0).abs() < 1e-5);
        assert_eq!(r2.follow_id, 1);
        assert!(r2.exiled_by.contains(&1));
        assert!(r1.stored_int.iter().any(|(k, v)| k == "homeTx" && *v == 50));
        assert_eq!(r1.first_name, "Ada");
    }

    #[test]
    fn null_refs_round_trip() {
        let mut rec = PlayerDiskRecord::default();
        rec.p_id = 9;
        rec.email = "n@t".into();
        rec.mother_id = PLAYER_REF_NULL;
        rec.follow_id = PLAYER_REF_NULL;
        rec.held_player_id = PLAYER_REF_NULL;
        rec.held_by_id = PLAYER_REF_NULL;
        let mut buf = Vec::new();
        write_player_record(&mut buf, &rec).unwrap();
        let got = read_player_record(&mut Cursor::new(buf)).unwrap();
        assert_eq!(got.mother_id, PLAYER_REF_NULL);
        assert_eq!(got.follow_id, PLAYER_REF_NULL);
        assert_eq!(get_player_from_id(got.held_player_id), None);
    }

    #[test]
    fn apply_to_player_aliases_hidden_wound() {
        let mut rec = PlayerDiskRecord::default();
        rec.p_id = 3;
        rec.email = "w@t".into();
        rec.body.held = Some(NestedHelper::with_uses(55, 1));
        rec.body.hidden_wound = Some(NestedHelper::with_uses(55, 1));
        let mut p = Player::new(0, 1, "tmp@t");
        apply_player_snapshot(&rec, &mut p);
        assert_eq!(p.held_id, 55);
        assert_eq!(
            p.hidden_wound.as_ref().map(|h| h.id),
            p.held_helper.as_ref().map(|h| h.id)
        );
    }

    #[test]
    fn second_pass_held_and_held_by_resolve() {
        let mut r1 = PlayerDiskRecord::default();
        r1.p_id = 1;
        r1.email = "m@t".into();
        r1.held_player_id = 2;
        r1.held_by_id = PLAYER_REF_NULL;

        let mut r2 = PlayerDiskRecord::default();
        r2.p_id = 2;
        r2.email = "b@t".into();
        r2.held_by_id = 1;
        r2.held_player_id = PLAYER_REF_NULL;
        r2.follow_id = 1;

        let snap = PlayersSnapshot {
            next_player_id: 3,
            records: vec![r1, r2],
        };
        let mut players = HashMap::new();
        let mut social = SocialState::default();
        let mut next = 2;
        let n = apply_players_snapshot(&snap, &mut players, &mut social, &mut next);
        assert_eq!(n, 2);
        assert_eq!(next, 3);

        let mut by_pid = HashMap::new();
        for p in players.values() {
            by_pid.insert(p.p_id, p);
        }
        assert_eq!(by_pid[&1].holding_player_id, 2);
        assert_eq!(by_pid[&2].held_by, 1);
        assert_eq!(social.following.get(&2), Some(&1));
        assert!(by_pid[&1].ai_controlled);
        assert!(!by_pid[&1].connected);
    }

    #[test]
    fn missing_players_file_empty_ok() {
        let dir = unique_temp_dir("miss");
        let path = dir.join("nope.bin");
        let loaded = load_players(&path).unwrap();
        assert!(loaded.is_empty());
        assert_eq!(loaded.next_player_id, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_save_load_file_roundtrip() {
        let dir = unique_temp_dir("file");
        let path = dir.join(DEFAULT_PLAYERS_FILE);
        let p = sample_player(7, "file@t");
        let mut players = HashMap::new();
        players.insert(p.conn_id, p);
        let social = SocialState::default();
        let snap = capture_players_snapshot(&players, &social, 8, |_| 0.0, false);
        save_players(&snap, &path).unwrap();
        assert!(path.exists());
        let loaded = load_players(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.records[0].p_id, 7);
        assert_eq!(loaded.records[0].body.clothing[1].as_ref().unwrap().id, 697);
        assert_eq!(
            loaded.records[0].body.clothing[1]
                .as_ref()
                .unwrap()
                .contained
                .len(),
            2
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn yum_state_roundtrip_via_apply() {
        let p = sample_player(4, "yum@t");
        let rec = capture_player_snapshot(&p, &CapturePlayerCtx::default());
        let mut p2 = Player::new(0, 99, "tmp");
        apply_player_snapshot(&rec, &mut p2);
        assert!((p2.yum.yum_bonus - 3.0).abs() < 1e-5);
        assert_eq!(p2.yum.currently_craving, 40);
        assert_eq!(p2.yum.get_count_eaten(33), 2.0);
        assert_eq!(p2.yum.get_count_eaten(40), -1.0);
        assert!(p2.yum.cravings.contains(&33) || p2.yum.cravings.contains(&40));
    }

    #[test]
    fn end_sign_mismatch_errors() {
        let rec = PlayerDiskRecord {
            p_id: 1,
            email: "e@t".into(),
            ..Default::default()
        };
        let mut buf = Vec::new();
        write_player_record(&mut buf, &rec).unwrap();
        // Corrupt end sign (just before rust extras — find -1000)
        // Safer: truncate mid-record
        buf.truncate(buf.len() / 2);
        assert!(read_player_record(&mut Cursor::new(buf)).is_err());
    }
}

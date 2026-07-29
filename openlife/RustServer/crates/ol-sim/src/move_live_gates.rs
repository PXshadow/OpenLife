//! Live grave-curse + close-enemy move-speed gates (Haxe `MoveHelper.calculateSpeed`).
//!
//! Chunk: **S-MOVE-LIVE-GATES** `grave_enemy_live`
//! Anchors:
//! - `PlayerAccount.hasCloseBlockingGrave` / `calculateCloseBlockingGraveFitness`
//! - `ServerSettings.MaxPlayersBeforeActivatingGraveCurse` / `GraveBlockingDistance`
//! - `isCursed` enter/clear hysteresis (`distance` vs `1.5 * distance`)
//! - `getClosePlayer(1.5, hostile, hasWeapon)` + `angryTime < 0`

// Haxe: ServerSettings.GraveBlockingDistance
pub const GRAVE_BLOCKING_DISTANCE: f32 = 40.0;
// Haxe: ServerSettings.MaxPlayersBeforeActivatingGraveCurse (default 0 → always when near)
pub const MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE: usize = 0;
// Haxe: ServerSettings.CombatAngryTimeBeforeAttack
pub const COMBAT_ANGRY_TIME_BEFORE_ATTACK: f32 = 5.0;
// Haxe: getClosePlayer(1.5, true, true)
pub const CLOSE_ENEMY_WEAPON_DISTANCE: f32 = 1.5;
// Haxe: clear curse only when beyond GraveBlockingDistance * 1.5
pub const GRAVE_CURSE_CLEAR_DISTANCE_MULT: f32 = 1.5;
// Haxe: calculateCloseBlockingGraveFitness threshold
pub const BLOCKING_GRAVE_FITNESS_THRESHOLD: f32 = 1.0;
// Haxe: per-grave fitness cap
pub const BLOCKING_GRAVE_FITNESS_CAP: f32 = 10.0;

/// Haxe `AiHelper.CalculateDistance` — squared Euclidean with optional torus wrap.
// Haxe: AiHelper.CalculateDistance
pub fn calculate_distance_sq(
    base_x: i32,
    base_y: i32,
    to_x: i32,
    to_y: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f64 {
    let mut diff_x = (to_x - base_x) as f64;
    let mut diff_y = (to_y - base_y) as f64;
    if wrap && map_w > 0 && map_h > 0 {
        let half_w = map_w as f64 / 2.0;
        let half_h = map_h as f64 / 2.0;
        if diff_x > half_w {
            diff_x -= map_w as f64;
        } else if diff_x < -half_w {
            diff_x += map_w as f64;
        }
        if diff_y > half_h {
            diff_y -= map_h as f64;
        } else if diff_y < -half_h {
            diff_y += map_h as f64;
        }
    }
    diff_x * diff_x + diff_y * diff_y
}

/// Haxe `PlayerAccount.calculateCloseBlockingGraveFitness`.
///
/// `graves` are account bone-grave tiles (Rust session `AccountRecord.graves` after bone filter).
// Haxe: PlayerAccount.calculateCloseBlockingGraveFitness
pub fn calculate_close_blocking_grave_fitness(
    tx: i32,
    ty: i32,
    graves: &[(i32, i32)],
    distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f32 {
    let dist = if distance.is_finite() && distance > 0.0 {
        distance
    } else {
        GRAVE_BLOCKING_DISTANCE
    };
    let dist_sq = (dist as f64) * (dist as f64);
    let mut fitness = 0.0f32;
    for &(gx, gy) in graves {
        let q = calculate_distance_sq(tx, ty, gx, gy, map_w, map_h, wrap);
        let mut tmp = (dist_sq / (1.0 + q)) as f32;
        if tmp > BLOCKING_GRAVE_FITNESS_CAP {
            tmp = BLOCKING_GRAVE_FITNESS_CAP;
        }
        fitness += tmp;
    }
    fitness
}

/// Haxe `PlayerAccount.hasCloseBlockingGrave` — fitness &gt; 1.
// Haxe: PlayerAccount.hasCloseBlockingGrave
#[inline]
pub fn has_close_blocking_grave(
    tx: i32,
    ty: i32,
    graves: &[(i32, i32)],
    distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    calculate_close_blocking_grave_fitness(tx, ty, graves, distance, map_w, map_h, wrap)
        > BLOCKING_GRAVE_FITNESS_THRESHOLD
}

/// Side-effect kind when `isCursed` flips during speed calc.
// Haxe: MoveHelper.calculateSpeed curse enter/clear
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveCurseTransition {
    None,
    /// Was not cursed → cursed (CU level 1 + sad say/emote).
    Entered,
    /// Was cursed → cleared (CU level 0 + happy say/emote).
    Cleared,
}

/// Pure resolution of grave-curse speed mali + `isCursed` state.
///
/// Matches Haxe:
/// ```text
/// if hasClose(default):
///   if living >= MaxPlayersBeforeActivatingGraveCurse:
///     speed mali; isCursed = true; Entered if was false
/// else:
///   if isCursed && !hasClose(1.5 * dist): clear; Cleared
/// ```
// Haxe: MoveHelper.calculateSpeed L210-231
pub fn resolve_grave_curse(
    is_cursed: bool,
    near_default_distance: bool,
    near_clear_distance: bool,
    living_players: usize,
    max_players_before: usize,
) -> (bool, bool, GraveCurseTransition) {
    if near_default_distance {
        let allow = living_players >= max_players_before;
        if allow {
            let trans = if !is_cursed {
                GraveCurseTransition::Entered
            } else {
                GraveCurseTransition::None
            };
            (true, true, trans)
        } else {
            // Population gate closed: no mali, leave isCursed unchanged.
            (false, is_cursed, GraveCurseTransition::None)
        }
    } else if is_cursed && !near_clear_distance {
        (false, false, GraveCurseTransition::Cleared)
    } else {
        // Outside default range but still inside clear hysteresis → keep cursed, no mali.
        (false, is_cursed, GraveCurseTransition::None)
    }
}

/// Haxe bow/ranged USE: refuse when `deadlyDistance > 1.9` and target animal within 1.5.
// Haxe: TransitionHelper.use L757-765
pub const RANGED_DEADLY_DISTANCE_THRESHOLD: f32 = 1.9;
/// Haxe min exact distance for ranged animal USE (`isCloseUseExact(..., 1.5)`).
// Haxe: TransitionHelper.use L761
pub const RANGED_MIN_USE_DISTANCE: f32 = 1.5;

/// Haxe `player.say('Too close...')` → uppercased in `sayHelper` → public PLAYER_SAYS.
// Haxe: TransitionHelper.use L762; GlobalPlayerInstance.sayHelper text.toUpperCase
pub const TOO_CLOSE_SAY: &str = "TOO CLOSE...";
/// Haxe `player.message = 'too close'` (debug / refuse reason; not wire).
// Haxe: TransitionHelper.use L763; killHelper has no message but USE sets it
pub const TOO_CLOSE_MESSAGE: &str = "too close";

/// Pending conn_id for ranged USE/KILL too-close public say (GPI-TOO-CLOSE).
/// `0` = none (conn_ids used by tests/sim start at 1).
// Haxe: TransitionHelper.use L761-764 / killHelper L4424 player.say('Too close...')
static LAST_TOO_CLOSE_SAY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Pending conn_id for Haxe `player.message = 'too close'` (debug refuse reason).
// Haxe: TransitionHelper.use L763 (non-wire)
static LAST_TOO_CLOSE_MESSAGE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Record pending too-close SAY (+ debug message) for USE/KILL refuse (live drains to PS).
// Haxe: TransitionHelper.use L762-763; killHelper L4424
pub fn note_too_close_say(conn_id: u64) {
    use std::sync::atomic::Ordering;
    // conn_id 0 is unused; treat as clear only via take/clear helpers.
    let id = if conn_id == 0 { 1 } else { conn_id };
    LAST_TOO_CLOSE_SAY.store(id, Ordering::SeqCst);
    // Haxe: player.message = 'too close' (debug refuse reason; not wire)
    LAST_TOO_CLOSE_MESSAGE.store(id, Ordering::SeqCst);
}

/// Take and clear pending too-close SAY conn_id (if any).
///
/// Does **not** clear the debug message channel — use [`take_too_close_message`].
pub fn take_too_close_say() -> Option<u64> {
    use std::sync::atomic::Ordering;
    let v = LAST_TOO_CLOSE_SAY.swap(0, Ordering::SeqCst);
    if v == 0 {
        None
    } else {
        Some(v)
    }
}

/// Take pending debug refuse reason for too-close (Haxe `player.message`).
///
/// Returns `(conn_id, TOO_CLOSE_MESSAGE)` when a refuse was noted.
// Haxe: TransitionHelper.use L763 player.message = 'too close'
pub fn take_too_close_message() -> Option<(u64, &'static str)> {
    use std::sync::atomic::Ordering;
    let v = LAST_TOO_CLOSE_MESSAGE.swap(0, Ordering::SeqCst);
    if v == 0 {
        None
    } else {
        Some((v, TOO_CLOSE_MESSAGE))
    }
}

/// Clear both too-close pending channels (test / refuse abort hygiene).
pub fn clear_too_close_pending() {
    use std::sync::atomic::Ordering;
    LAST_TOO_CLOSE_SAY.store(0, Ordering::SeqCst);
    LAST_TOO_CLOSE_MESSAGE.store(0, Ordering::SeqCst);
}

/// Haxe `MoveHelper.calculateExactQuadDistance` core — squared float distance with optional wrap.
///
/// Mirrors `transformFloatX/Y` half-map wrap when `wrap` (absolute exact positions, gx/gy=0).
// Haxe: MoveHelper.calculateExactQuadDistance + WorldMap.transformFloatX/Y
#[inline]
pub fn calculate_exact_quad_distance_f(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f64 {
    let mut dx = ax - bx;
    let mut dy = ay - by;
    if wrap && map_w > 0 && map_h > 0 {
        let half_w = map_w as f64 / 2.0;
        let half_h = map_h as f64 / 2.0;
        if dx > half_w {
            dx -= map_w as f64;
        } else if dx < -half_w {
            dx += map_w as f64;
        }
        if dy > half_h {
            dy -= map_h as f64;
        } else if dy < -half_h {
            dy += map_h as f64;
        }
    }
    dx * dx + dy * dy
}

/// Haxe `isCloseUseExact`: quad distance ≤ max_distance² (integer tile positions).
// Haxe: MoveHelper.isCloseUseExact
#[inline]
pub fn is_close_use_exact(ax: i32, ay: i32, bx: i32, by: i32, max_distance: f32) -> bool {
    is_close_use_exact_f(ax as f64, ay as f64, bx as f64, by as f64, max_distance)
}

/// Integer-tile exact range with map wrap.
// Haxe: MoveHelper.isCloseUseExact + transformFloat
#[inline]
pub fn is_close_use_exact_wrap(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    max_distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    is_close_use_exact_f_wrap(
        ax as f64,
        ay as f64,
        bx as f64,
        by as f64,
        max_distance,
        map_w,
        map_h,
        wrap,
    )
}

/// Haxe `isCloseUseExact` with float exact move positions (`exactTx` / `exactTy`).
// Haxe: MoveHelper.isCloseUseExact / isCloseToPlayerUseExact
#[inline]
pub fn is_close_use_exact_f(ax: f64, ay: f64, bx: f64, by: f64, max_distance: f32) -> bool {
    is_close_use_exact_f_wrap(ax, ay, bx, by, max_distance, 0, 0, false)
}

/// Float exact range with optional torus wrap (Haxe `calculateExactQuadDistance`).
// Haxe: MoveHelper.isCloseUseExact / calculateExactQuadDistance
#[inline]
pub fn is_close_use_exact_f_wrap(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    max_distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    let max_d = if max_distance.is_finite() && max_distance > 0.0 {
        max_distance
    } else {
        1.0
    };
    let q = calculate_exact_quad_distance_f(ax, ay, bx, by, map_w, map_h, wrap);
    q <= (max_d as f64) * (max_d as f64)
}

/// Haxe killHelper / TransitionHelper ranged min-range: deadly held + exact ≤ 1.5.
///
/// Shared core for player-target kill (no animal gate) and animal USE (with animal gate).
/// When true, caller should refuse and public-say `Too close...`.
// Haxe: GlobalPlayerInstance.killHelper L4420-4428 (player targets, no animal check)
#[inline]
pub fn refuse_ranged_kill_too_close(
    held_deadly_distance: f32,
    player_x: f64,
    player_y: f64,
    target_x: f64,
    target_y: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if !(held_deadly_distance.is_finite()
        && held_deadly_distance > RANGED_DEADLY_DISTANCE_THRESHOLD)
    {
        return false;
    }
    is_close_use_exact_f_wrap(
        player_x,
        player_y,
        target_x,
        target_y,
        RANGED_MIN_USE_DISTANCE,
        map_w,
        map_h,
        wrap,
    )
}

/// Haxe TransitionHelper ranged USE refuse: deadly held + animal target too close.
///
/// When true, caller should refuse USE (Haxe: `say('Too close...')`).
// Haxe: TransitionHelper.use L757-765
#[inline]
pub fn refuse_ranged_use_too_close(
    held_deadly_distance: f32,
    target_is_animal: bool,
    player_x: f64,
    player_y: f64,
    target_x: f64,
    target_y: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if !target_is_animal {
        return false;
    }
    refuse_ranged_kill_too_close(
        held_deadly_distance,
        player_x,
        player_y,
        target_x,
        target_y,
        map_w,
        map_h,
        wrap,
    )
}

/// Snapshot of another living player for close-hostile weapon scan.
// Haxe: GlobalPlayerInstance.getClosePlayer hostile+hasWeapon
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosePlayerCandidate {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// Exact move position (defaults to tile center when not tracking sub-tile).
    pub exact_x: f32,
    pub exact_y: f32,
    pub deleted: bool,
    pub holding_weapon: bool,
    /// Haxe `candidate.isFriendly(observer)` — leadership ally and not last-attack.
    pub is_friendly: bool,
}

impl ClosePlayerCandidate {
    /// Build from integer tile (exact = tile coords as f32).
    pub fn from_tile(
        p_id: i32,
        x: i32,
        y: i32,
        deleted: bool,
        holding_weapon: bool,
        is_friendly: bool,
    ) -> Self {
        Self {
            p_id,
            x,
            y,
            exact_x: x as f32,
            exact_y: y as f32,
            deleted,
            holding_weapon,
            is_friendly,
        }
    }
}

/// Haxe `getClosePlayer(maxDistance, hostile=true, hasWeapon=true)` existence check.
///
/// Uses float exact positions when set on candidates (Haxe `isCloseToPlayerUseExact`).
// Haxe: GlobalPlayerInstance.getClosePlayer
pub fn has_close_hostile_with_weapon(
    observer_x: i32,
    observer_y: i32,
    observer_p_id: i32,
    candidates: &[ClosePlayerCandidate],
    max_distance: f32,
) -> bool {
    has_close_hostile_with_weapon_exact(
        observer_x as f64,
        observer_y as f64,
        observer_p_id,
        candidates,
        max_distance,
    )
}

/// Float-exact variant (Haxe moveHelper.exactTx/exactTy).
// Haxe: GlobalPlayerInstance.getClosePlayer + isCloseToPlayerUseExact
pub fn has_close_hostile_with_weapon_exact(
    observer_x: f64,
    observer_y: f64,
    observer_p_id: i32,
    candidates: &[ClosePlayerCandidate],
    max_distance: f32,
) -> bool {
    for c in candidates {
        if c.deleted || c.p_id == observer_p_id {
            continue;
        }
        if max_distance > 0.0
            && !is_close_use_exact_f(
                observer_x,
                observer_y,
                c.exact_x as f64,
                c.exact_y as f64,
                max_distance,
            )
        {
            continue;
        }
        // hostile: skip friendly (candidate.isFriendly(observer))
        if c.is_friendly {
            continue;
        }
        if !c.holding_weapon {
            continue;
        }
        return true;
    }
    false
}

/// Haxe: close hostile weapon **and** `angryTime < 0` → apply speed factor.
// Haxe: MoveHelper.calculateSpeed L249-251
#[inline]
pub fn close_hostile_weapon_speed_active(angry_time: f32, has_close_hostile_weapon: bool) -> bool {
    has_close_hostile_weapon && angry_time < 0.0
}

/// Haxe `isFriendly` subset without last-attack bookkeeping (ally only).
// Haxe: GlobalPlayerInstance.isFriendly (ally part)
#[inline]
pub fn is_friendly_ally_only(is_ally: bool) -> bool {
    is_ally
}

/// Haxe `GlobalPlayerInstance.isFriendly(player)`:
/// `isAlly(player) && lastAttackedPlayer != player && lastPlayerAttackedMe != player`.
///
/// Call with **candidate's** last-attack ids and **observer** as `other_p_id`
/// (`candidate.isFriendly(observer)` in `getClosePlayer`).
// Haxe: GlobalPlayerInstance.isFriendly
#[inline]
pub fn is_friendly(
    is_leadership_ally: bool,
    last_attacked_player_id: i32,
    last_player_attacked_me_id: i32,
    other_p_id: i32,
) -> bool {
    is_leadership_ally
        && last_attacked_player_id != other_p_id
        && last_player_attacked_me_id != other_p_id
}

/// Haxe `ObjectData.isWeapon` — `deadlyDistance > 0` (includes bloody weapons).
// Haxe: ObjectData.isWeapon
#[inline]
pub fn is_weapon_deadly_distance(deadly_distance: f32) -> bool {
    deadly_distance.is_finite() && deadly_distance > 0.0
}

/// Haxe `removeDeletedGraves` + `isBoneGrave` filter for account grave tiles.
///
/// Drops world `id == 0` (deleted) and non-bone objects. Does **not** keep empty tiles.
// Haxe: PlayerAccount.removeDeletedGraves + isBoneGrave
pub fn account_blocking_grave_tiles_from_ids(
    graves: &[(i32, i32)],
    world_object_at: impl Fn(i32, i32) -> i32,
    is_bone_grave: impl Fn(i32) -> bool,
) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for &(gx, gy) in graves {
        let id = world_object_at(gx, gy);
        // Haxe: id==0 removed; only isBoneGrave counted
        if id != 0 && is_bone_grave(id) {
            out.push((gx, gy));
        }
    }
    out
}

/// Count living connection-like players (Haxe `GetNumberLifingPlayers` over connections).
// Haxe: GlobalPlayerInstance.GetNumberLifingPlayers
#[inline]
pub fn living_connection_player_count(players: impl Iterator<Item = (bool, bool)>) -> usize {
    // (deleted, connected)
    players.filter(|(deleted, connected)| !*deleted && *connected).count()
}

/// Format CU wire body (Haxe `Connection.SendCurseToAll`).
// Haxe: Connection.SendCurseToAll → ClientTag CURSED = "CU"
pub fn format_cursed_message(p_id: i32, level: i32) -> String {
    format!("CU\n{p_id} {level}\n#")
}

/// Emote indices for curse enter/clear (Haxe Emote.sad / Emote.happy).
pub const CURSE_ENTER_EMOTE_INDEX: i32 = 3; // SAD
pub const CURSE_CLEAR_EMOTE_INDEX: i32 = 0; // HAPPY

/// Private say lines when curse state flips (Haxe `p.say(..., true)`).
pub const CURSE_ENTER_SAY: &str = "My bones are near im cursed...";
pub const CURSE_CLEAR_SAY: &str = "Im far away from my bones...";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_gates_blocking_grave_fitness_near_and_far() {
        // At same tile: dist 0 → fitness = 40² / 1 = 1600 → cap 10 per grave
        let f = calculate_close_blocking_grave_fitness(
            0,
            0,
            &[(0, 0)],
            GRAVE_BLOCKING_DISTANCE,
            100,
            100,
            false,
        );
        assert!((f - 10.0).abs() < 1e-4, "f={f}");
        assert!(has_close_blocking_grave(
            0,
            0,
            &[(0, 0)],
            GRAVE_BLOCKING_DISTANCE,
            100,
            100,
            false
        ));
        // Far: dx=200 → q=40000, fitness = 1600/40001 ≪ 1
        let far = calculate_close_blocking_grave_fitness(
            0,
            0,
            &[(200, 0)],
            GRAVE_BLOCKING_DISTANCE,
            1000,
            1000,
            false,
        );
        assert!(far < 1.0, "far={far}");
        assert!(!has_close_blocking_grave(
            0,
            0,
            &[(200, 0)],
            GRAVE_BLOCKING_DISTANCE,
            1000,
            1000,
            false
        ));
    }

    #[test]
    fn live_gates_grave_curse_enter_clear_hysteresis() {
        // Enter: near default + population gate open
        let (mali, cursed, t) = resolve_grave_curse(false, true, true, 1, 0);
        assert!(mali && cursed);
        assert_eq!(t, GraveCurseTransition::Entered);

        // Stay cursed near: no re-enter transition
        let (mali2, cursed2, t2) = resolve_grave_curse(true, true, true, 1, 0);
        assert!(mali2 && cursed2);
        assert_eq!(t2, GraveCurseTransition::None);

        // Outside default but inside clear band: keep cursed, no mali
        let (mali3, cursed3, t3) = resolve_grave_curse(true, false, true, 1, 0);
        assert!(!mali3 && cursed3);
        assert_eq!(t3, GraveCurseTransition::None);

        // Beyond clear distance: clear
        let (mali4, cursed4, t4) = resolve_grave_curse(true, false, false, 1, 0);
        assert!(!mali4 && !cursed4);
        assert_eq!(t4, GraveCurseTransition::Cleared);

        // Population gate closed (need ≥2 living)
        let (mali5, cursed5, t5) = resolve_grave_curse(false, true, true, 1, 2);
        assert!(!mali5 && !cursed5);
        assert_eq!(t5, GraveCurseTransition::None);
    }

    #[test]
    fn live_gates_close_hostile_weapon() {
        let cands = [
            ClosePlayerCandidate::from_tile(2, 1, 0, false, true, false),
            ClosePlayerCandidate::from_tile(3, 0, 0, false, true, true),
        ];
        assert!(has_close_hostile_with_weapon(
            0,
            0,
            1,
            &cands,
            CLOSE_ENEMY_WEAPON_DISTANCE
        ));
        // Ally with weapon ignored
        assert!(!has_close_hostile_with_weapon(
            0,
            0,
            1,
            &cands[1..],
            CLOSE_ENEMY_WEAPON_DISTANCE
        ));
        // Far enemy ignored
        let far = [ClosePlayerCandidate::from_tile(9, 10, 10, false, true, false)];
        assert!(!has_close_hostile_with_weapon(
            0,
            0,
            1,
            &far,
            CLOSE_ENEMY_WEAPON_DISTANCE
        ));
    }

    #[test]
    fn live_gates_angry_time_required_for_enemy_speed() {
        assert!(!close_hostile_weapon_speed_active(5.0, true));
        assert!(!close_hostile_weapon_speed_active(-0.1, false));
        assert!(close_hostile_weapon_speed_active(-0.1, true));
        assert!(close_hostile_weapon_speed_active(-1.0, true));
    }

    #[test]
    fn live_gates_format_cursed_cu() {
        assert_eq!(format_cursed_message(42, 1), "CU\n42 1\n#");
        assert_eq!(format_cursed_message(42, 0), "CU\n42 0\n#");
    }

    #[test]
    fn live_gates_wrap_distance_shorter_across_seam() {
        // map 100 wide: 0 → 99 is dist 1 wrapped vs 99 unwrapped
        let wrap = calculate_distance_sq(0, 0, 99, 0, 100, 100, true);
        let no = calculate_distance_sq(0, 0, 99, 0, 100, 100, false);
        assert!((wrap - 1.0).abs() < 1e-6, "wrap={wrap}");
        assert!((no - 99.0 * 99.0).abs() < 1e-6, "no={no}");
    }

    #[test]
    fn live_gates_is_friendly_last_attack() {
        // Leadership ally without last-attack history → friendly
        assert!(is_friendly(true, 0, 0, 2));
        // Ally who last attacked observer → not friendly
        assert!(!is_friendly(true, 2, 0, 2));
        // Ally who was last attacked by observer (on candidate book) → not friendly
        assert!(!is_friendly(true, 0, 2, 2));
        // Non-ally never friendly
        assert!(!is_friendly(false, 0, 0, 2));
        assert!(is_friendly_ally_only(true));
        assert!(!is_friendly_ally_only(false));
    }

    #[test]
    fn live_gates_weapon_deadly_distance() {
        assert!(!is_weapon_deadly_distance(0.0));
        assert!(!is_weapon_deadly_distance(-1.0));
        assert!(is_weapon_deadly_distance(0.1));
        assert!(is_weapon_deadly_distance(1.5));
        assert!(is_weapon_deadly_distance(4.0));
    }

    // Haxe: isCloseUseExact inclusive dist² == max²; wrap via transformFloat
    #[test]
    fn is_close_use_exact_boundary_and_wrap() {
        // dist² == 2.25 inclusive at max=1.5
        assert!(is_close_use_exact_f(0.0, 0.0, 1.5, 0.0, 1.5));
        assert!(!is_close_use_exact_f(0.0, 0.0, 1.5001, 0.0, 1.5));
        // wrap: 0 and 99 on map 100
        assert!(is_close_use_exact_f_wrap(
            0.0, 0.0, 99.0, 0.0, 1.5, 100, 100, true
        ));
        assert!(!is_close_use_exact_f_wrap(
            0.0, 0.0, 99.0, 0.0, 1.5, 100, 100, false
        ));
        let q = calculate_exact_quad_distance_f(0.0, 0.0, 99.0, 0.0, 100, 100, true);
        assert!((q - 1.0).abs() < 1e-9, "q={q}");
    }

    // Haxe: TransitionHelper.use bow min-range refuse
    #[test]
    fn refuse_ranged_use_too_close_bow_animal() {
        // deadly 4, animal at (1,0) from player → too close
        assert!(refuse_ranged_use_too_close(
            4.0, true, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // same but not animal → ok (USE path only)
        assert!(!refuse_ranged_use_too_close(
            4.0, false, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // knife deadly 1.5 ≤ 1.9 → no min-range refuse
        assert!(!refuse_ranged_use_too_close(
            1.5, true, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // animal at range 3 (exact) while deadly 4 → allow
        assert!(!refuse_ranged_use_too_close(
            4.0, true, 0.0, 0.0, 3.0, 0.0, 32, 32, false
        ));
        // boundary: exact dist 1.5 inclusive refuse
        assert!(refuse_ranged_use_too_close(
            4.0, true, 0.0, 0.0, 1.5, 0.0, 32, 32, false
        ));
    }

    // Haxe: killHelper L4420-4428 bow min-range on player targets (no animal check)
    #[test]
    fn refuse_ranged_kill_too_close_bow_player() {
        // deadly 4, player at (1,0) → too close (unlike USE, no animal required)
        assert!(refuse_ranged_kill_too_close(
            4.0, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // knife deadly 1.5 ≤ 1.9 → allow melee
        assert!(!refuse_ranged_kill_too_close(
            1.5, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // bare hands / non-ranged
        assert!(!refuse_ranged_kill_too_close(
            0.0, 0.0, 0.0, 1.0, 0.0, 32, 32, false
        ));
        // bow at exact 3 → allow shot
        assert!(!refuse_ranged_kill_too_close(
            4.0, 0.0, 0.0, 3.0, 0.0, 32, 32, false
        ));
        // boundary exact 1.5 inclusive refuse; just beyond allow
        assert!(refuse_ranged_kill_too_close(
            4.0, 0.0, 0.0, 1.5, 0.0, 32, 32, false
        ));
        assert!(!refuse_ranged_kill_too_close(
            4.0, 0.0, 0.0, 1.5001, 0.0, 32, 32, false
        ));
    }

    // Haxe: TransitionHelper.use L762 pending say channel (GPI-TOO-CLOSE)
    #[test]
    fn too_close_say_note_take() {
        let _ = take_too_close_say();
        let _ = take_too_close_message();
        assert!(take_too_close_say().is_none());
        assert!(take_too_close_message().is_none());
        note_too_close_say(42);
        assert_eq!(take_too_close_say(), Some(42));
        assert_eq!(take_too_close_message(), Some((42, TOO_CLOSE_MESSAGE)));
        assert!(take_too_close_say().is_none());
        assert!(take_too_close_message().is_none());
        assert_eq!(TOO_CLOSE_SAY, "TOO CLOSE...");
        assert_eq!(TOO_CLOSE_MESSAGE, "too close");
    }

    #[test]
    fn live_gates_account_blocking_drops_id_zero() {
        let graves = [(0, 0), (1, 0), (2, 0)];
        // world: tile0 empty, tile1 bone 87, tile2 non-bone 1
        let tiles = account_blocking_grave_tiles_from_ids(
            &graves,
            |x, _y| match x {
                0 => 0,
                1 => 87,
                2 => 1,
                _ => 0,
            },
            |id| id == 87,
        );
        assert_eq!(tiles, vec![(1, 0)]);
    }

    #[test]
    fn live_gates_living_connection_count() {
        let rows = [(false, true), (false, false), (true, true), (false, true)];
        assert_eq!(
            living_connection_player_count(rows.into_iter()),
            2
        );
    }

    #[test]
    fn live_gates_exact_float_distance() {
        // Integer tiles far (dx=2), but exact positions within 1.5
        let mut c = ClosePlayerCandidate::from_tile(2, 2, 0, false, true, false);
        c.exact_x = 1.2;
        c.exact_y = 0.0;
        assert!(has_close_hostile_with_weapon_exact(
            0.0,
            0.0,
            1,
            &[c],
            CLOSE_ENEMY_WEAPON_DISTANCE
        ));
        // Exact far even if tile says close
        let mut far = ClosePlayerCandidate::from_tile(3, 1, 0, false, true, false);
        far.exact_x = 5.0;
        far.exact_y = 0.0;
        assert!(!has_close_hostile_with_weapon_exact(
            0.0,
            0.0,
            1,
            &[far],
            CLOSE_ENEMY_WEAPON_DISTANCE
        ));
    }
}

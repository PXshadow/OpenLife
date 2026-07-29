//! Continuous AI follow walk (**AI-FOLLOW-WALK** / continuous_follow)
//! + empty-sticky acquire (**AI-FOLLOW-ACQUIRE** / auto_follow).
//!
//! Haxe `AiBase.isMovingToPlayer` + ordered-follow auto-clear + ally `Goto(speaker)`.
//! Sticky target is `Player.ai_follow_p_id` (from **AI-LLM-APPLY** / scripted FOLLOW).
//! When sticky is empty: child-mother `getFollowPlayer` or `AutoFollowPlayer` closest.
//! Pure decision helpers; live pathfind/start is wired in `lib` + npc.

use crate::ai_goals::priority_ladder::{
    baby_hungry_follow_tiles, child_with_mother_follow_tiles, is_child_and_has_mother_ex,
    is_moving_to_player_needed, ordered_follow_max_tiles, player_quad_dist, wounded_follow_tiles,
    MIN_AGE_TO_EAT,
};

/// Max ordered-follow duration before Haxe forces `autoStopFollow = true` (5 min).
// Haxe: AiBase.doTimeStuffHelper time > 60 * 5
pub const ORDERED_FOLLOW_MAX_SECS: f32 = 60.0 * 5.0;

/// Age gate: when `autoStopFollow` and age > MinAgeToEat * 2, clear follow target.
// Haxe: age > ServerSettings.MinAgeToEat * 2
pub const AUTO_STOP_FOLLOW_CLEAR_AGE: f32 = MIN_AGE_TO_EAT * 2.0;

/// Default AutoFollowPlayer search radius (Haxe getClosestPlayer(20, …)).
// Haxe: AiBase.isMovingToPlayer getClosestPlayer(20)
pub const AUTO_FOLLOW_SEARCH_TILES: i32 = 20;

/// Haxe `ServerSettings.AutoFollowPlayer` default (false until enabled in config).
// Haxe: ServerSettings.AutoFollowPlayer = false
pub const AUTO_FOLLOW_PLAYER_DEFAULT: bool = false;

/// Path step cap for AI follow Goto (Haxe calculateNewMovements count > 10).
pub const FOLLOW_PATH_STEP_CAP: usize = 10;

/// Snapshot of sticky AI follow fields on a player.
// Haxe: AiBase.playerToFollow / autoStopFollow / timeStartedToFolow
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiFollowSticky {
    pub follow_p_id: i32,
    pub auto_stop_follow: bool,
    pub follow_started_sim_time: f32,
}

impl Default for AiFollowSticky {
    fn default() -> Self {
        Self {
            follow_p_id: 0,
            auto_stop_follow: true,
            follow_started_sim_time: 0.0,
        }
    }
}

/// Mutation plan for sticky follow auto-clear rules (no world I/O).
// Haxe: AiBase.doTimeStuffHelper playerToFollow auto-clear ~560–568
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowStickyClearPlan {
    /// Force `auto_stop_follow = true` (ordered follow timed out).
    pub set_auto_stop: bool,
    /// Clear `follow_p_id` to 0.
    pub clear_target: bool,
}

/// Apply ordered-follow timeout + age clear rules.
///
/// 1. Ordered (`!auto_stop`): after 5 min → set auto_stop.
/// 2. Loose (`auto_stop`) and age > MinAgeToEat*2 → clear target.
// Haxe: AiBase.doTimeStuffHelper L560–568
pub fn plan_follow_sticky_clear(
    sticky: &AiFollowSticky,
    age: f32,
    now_sim: f32,
) -> FollowStickyClearPlan {
    plan_follow_sticky_clear_ex(sticky, age, now_sim, MIN_AGE_TO_EAT)
}

/// Same as [`plan_follow_sticky_clear`] with live MinAgeToEat (clear when age > min*2).
// Haxe: ServerSettings.MinAgeToEat * 2 — C-SS-MIN-AGE-AI
pub fn plan_follow_sticky_clear_ex(
    sticky: &AiFollowSticky,
    age: f32,
    now_sim: f32,
    min_age_to_eat: f32,
) -> FollowStickyClearPlan {
    if sticky.follow_p_id <= 0 {
        return FollowStickyClearPlan {
            set_auto_stop: false,
            clear_target: false,
        };
    }
    let mut set_auto_stop = false;
    let mut auto_stop = sticky.auto_stop_follow;
    if !auto_stop {
        let elapsed = (now_sim - sticky.follow_started_sim_time).max(0.0);
        if elapsed > ORDERED_FOLLOW_MAX_SECS {
            set_auto_stop = true;
            auto_stop = true;
        }
    }
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
    let clear_age = min_age * 2.0;
    let clear_target = auto_stop && age > clear_age;
    FollowStickyClearPlan {
        set_auto_stop,
        clear_target,
    }
}

/// Apply [`FollowStickyClearPlan`] onto sticky fields.
pub fn apply_follow_sticky_clear(sticky: &mut AiFollowSticky, plan: FollowStickyClearPlan) {
    if plan.set_auto_stop {
        sticky.auto_stop_follow = true;
    }
    if plan.clear_target {
        sticky.follow_p_id = 0;
    }
}

/// Max tile radius for continuous follow walk (ordered vs loose).
// Haxe: isMovingToPlayer(autoStopFollow ? 10 : 5)
pub fn follow_max_tiles_for_sticky(auto_stop_follow: bool) -> i32 {
    ordered_follow_max_tiles(auto_stop_follow)
}

/// Specialized + general max tiles for continuous `isMovingToPlayer` walk.
///
/// Mirrors Haxe `doTimeStuffHelper` priority order before the late general call:
/// 1. baby hungry (`age < MinAgeToEat && hungry`) → 5
/// 2. child with mother → nice 2 / else 4
/// 3. wounded / yellow fever → 2
/// 4. else ordered 5 / loose 10
// Haxe: AiBase.doTimeStuffHelper L523–595 isMovingToPlayer distance args
// AI-FOLLOW-ACQUIRE / continuous_follow bands
pub fn follow_max_tiles_for_context(
    age: f32,
    is_hungry: bool,
    has_living_mother: bool,
    is_nice_baby: bool,
    is_wounded_or_fever: bool,
    auto_stop_follow: bool,
) -> i32 {
    follow_max_tiles_for_context_ex(
        age,
        is_hungry,
        has_living_mother,
        is_nice_baby,
        is_wounded_or_fever,
        auto_stop_follow,
        MIN_AGE_TO_EAT,
    )
}

/// Same as [`follow_max_tiles_for_context`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn follow_max_tiles_for_context_ex(
    age: f32,
    is_hungry: bool,
    has_living_mother: bool,
    is_nice_baby: bool,
    is_wounded_or_fever: bool,
    auto_stop_follow: bool,
    min_age_to_eat: f32,
) -> i32 {
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
    if age < min_age && is_hungry {
        return baby_hungry_follow_tiles();
    }
    if is_child_and_has_mother_ex(age, has_living_mother, min_age) {
        return child_with_mother_follow_tiles(is_nice_baby);
    }
    if is_wounded_or_fever {
        return wounded_follow_tiles();
    }
    follow_max_tiles_for_sticky(auto_stop_follow)
}

/// Whether AI should say follow target name while walking toward them.
// Haxe: AiBase.isMovingToPlayer L8318 age > MinAgeToEat || shouldDebugSay()
// AI-FOLLOW-ACQUIRE / continuous_follow debug say
pub fn should_say_follow_target_name(age: f32, debug_say: bool) -> bool {
    should_say_follow_target_name_ex(age, debug_say, MIN_AGE_TO_EAT)
}

/// Same as [`should_say_follow_target_name`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn should_say_follow_target_name_ex(age: f32, debug_say: bool, min_age_to_eat: f32) -> bool {
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
    age > min_age || debug_say
}

/// Half-range for random stand-off offset around follow target.
///
/// Haxe uses squared maxDistance: `dist = maxDistance_sq >= 9 ? 2 : 1`.
// Haxe: AiBase.isMovingToPlayer L8313
pub fn follow_stand_half_range(max_distance_tiles: i32) -> i32 {
    let max_q = max_distance_tiles.max(0) * max_distance_tiles.max(0);
    if max_q >= 9 {
        2
    } else {
        1
    }
}

/// Deterministic pseudo-random offset in `[-half, half]` from seed bits.
///
/// Replaces Haxe `WorldMap.calculateRandomInt(2*dist) - dist` without global RNG.
pub fn follow_offset_from_seed(seed: u32, half: i32) -> (i32, i32) {
    let half = half.max(0);
    if half == 0 {
        return (0, 0);
    }
    let span = 2 * half + 1;
    let rx = (seed % span as u32) as i32 - half;
    let ry = ((seed / 7) % span as u32) as i32 - half;
    (rx, ry)
}

/// Goal tile beside follow target (Haxe `player.tx + randX, player.ty + randY`).
// Haxe: AiBase.isMovingToPlayer gotoAdv(playerToFollow.tx + randX, …)
pub fn follow_goal_xy(
    target_x: i32,
    target_y: i32,
    max_distance_tiles: i32,
    seed: u32,
) -> (i32, i32) {
    let half = follow_stand_half_range(max_distance_tiles);
    let (rx, ry) = follow_offset_from_seed(seed, half);
    (target_x + rx, target_y + ry)
}

/// Ally / startFollowing immediate Goto: stand one tile east of speaker.
// Haxe: startFollowingPlayer Goto(player.tx + 1 - gx, player.ty - gy)
// Haxe: ally post-say Goto(speaker) / MOVE! Goto(player.tx + 1)
pub fn ally_goto_speaker_xy(speaker_x: i32, speaker_y: i32) -> (i32, i32) {
    (speaker_x + 1, speaker_y)
}

/// Outcome of pure continuous-follow decision (no pathfind).
// Haxe: AiBase.isMovingToPlayer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowWalkDecision {
    /// No sticky / no live target.
    NoTarget,
    /// Sticky pointed at a deleted player — clear sticky.
    TargetDeleted,
    /// Within maxDistance² — stay put (Haxe returns false).
    CloseEnough,
    /// Too far — walk to goal (caller pathfinds).
    WalkTo { goal_x: i32, goal_y: i32 },
}

/// Pure `isMovingToPlayer` decision given sticky + live target coords.
///
/// - Clears conceptually when target deleted (`TargetDeleted`).
/// - Uses squared tile distance vs `max_distance_tiles`.
/// - Goal includes stand-off random offset from `seed`.
// Haxe: AiBase.isMovingToPlayer
pub fn decide_follow_walk(
    sticky_follow_p_id: i32,
    target: Option<FollowTargetSnap>,
    ai_x: i32,
    ai_y: i32,
    max_distance_tiles: i32,
    seed: u32,
) -> FollowWalkDecision {
    if sticky_follow_p_id <= 0 {
        return FollowWalkDecision::NoTarget;
    }
    let Some(t) = target else {
        return FollowWalkDecision::TargetDeleted;
    };
    if t.deleted || t.p_id != sticky_follow_p_id {
        return FollowWalkDecision::TargetDeleted;
    }
    let qd = player_quad_dist(ai_x, ai_y, t.x, t.y);
    if !is_moving_to_player_needed(qd, max_distance_tiles) {
        return FollowWalkDecision::CloseEnough;
    }
    let (gx, gy) = follow_goal_xy(t.x, t.y, max_distance_tiles, seed);
    FollowWalkDecision::WalkTo {
        goal_x: gx,
        goal_y: gy,
    }
}

/// Live target player snapshot for pure decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowTargetSnap {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
}

/// Whether continuous walk should hold the AI tick (Haxe returns true from isMovingToPlayer).
// Haxe: if (isMovingToPlayer(...)) return;
pub fn follow_walk_holds_tick(d: FollowWalkDecision) -> bool {
    matches!(d, FollowWalkDecision::WalkTo { .. })
}

/// Seed from sim time + AI p_id for deterministic stand-off.
pub fn follow_seed(sim_time: f32, ai_p_id: i32) -> u32 {
    let t = (sim_time * 10.0) as i32;
    (t.wrapping_mul(31).wrapping_add(ai_p_id.wrapping_mul(17))) as u32
}

/// Cap path steps for follow Goto (first N per-step deltas).
pub fn truncate_follow_path_steps(steps: &[(i32, i32)], cap: usize) -> Vec<(i32, i32)> {
    steps.iter().copied().take(cap.max(1)).collect()
}

/// Whether sensors should mark ordered follow active (priority ladder).
// Haxe: ordered follow when playerToFollow && !autoStopFollow
pub fn ordered_follow_sensor(sticky: &AiFollowSticky) -> bool {
    sticky.follow_p_id > 0 && !sticky.auto_stop_follow
}

/// Whether sensors should mark loose follow / isMovingToPlayer band.
// Haxe: follow_player when playerToFollow non-null after clears
pub fn follow_player_sensor(sticky: &AiFollowSticky) -> bool {
    sticky.follow_p_id > 0
}

// ── AI-FOLLOW-ACQUIRE / auto_follow ─────────────────────────────────────────

/// Candidate for Haxe `getClosestPlayer` (human connections first, then AIs).
// Haxe: GlobalPlayerInstance.getClosestPlayer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoFollowCandidate {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// True for human Connection bodies; false for permanent/AI bodies.
    // Haxe: Connection.getConnections vs getAis
    pub is_human: bool,
    pub deleted: bool,
}

/// Source of a newly acquired sticky walk target.
// Haxe: AiBase.isMovingToPlayer playerToFollow = null branch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoFollowAcquireSource {
    /// Child under MinAgeToEat with living leadership `followPlayer` (mother).
    ChildMother,
    /// `ServerSettings.AutoFollowPlayer` + closest in radius.
    ClosestPlayer,
}

/// Result of empty-sticky acquire (Haxe sets `playerToFollow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoFollowAcquire {
    pub follow_p_id: i32,
    pub source: AutoFollowAcquireSource,
}

/// Haxe `getClosestPlayer(maxDistance, onlyHuman)`.
///
/// - Skips self (`exclude_p_id`) and deleted.
/// - Humans first (Connection loop), then AIs unless `only_human`.
/// - Quad distance; on ties last equal wins (Haxe `>` continue).
// Haxe: GlobalPlayerInstance.getClosestPlayer L2962–3002
pub fn get_closest_player_for_auto_follow(
    from_x: i32,
    from_y: i32,
    max_distance: i32,
    only_human: bool,
    exclude_p_id: i32,
    candidates: &[AutoFollowCandidate],
) -> Option<i32> {
    if max_distance < 0 {
        return None;
    }
    let quad_max = (max_distance as f32) * (max_distance as f32);
    let mut best_dist = quad_max;
    let mut best: Option<i32> = None;

    // Pass 1: humans (Haxe Connection.getConnections)
    for c in candidates {
        if c.deleted || !c.is_human || c.p_id == exclude_p_id || c.p_id <= 0 {
            continue;
        }
        let qd = player_quad_dist(from_x, from_y, c.x, c.y);
        // Haxe: if (tmpQuadDistance > bestDistance) continue; else update
        if qd > best_dist {
            continue;
        }
        best_dist = qd;
        best = Some(c.p_id);
    }
    if only_human {
        return best;
    }
    // Pass 2: AIs can replace if closer or equal (Haxe getAis)
    for c in candidates {
        if c.deleted || c.is_human || c.p_id == exclude_p_id || c.p_id <= 0 {
            continue;
        }
        let qd = player_quad_dist(from_x, from_y, c.x, c.y);
        if qd > best_dist {
            continue;
        }
        best_dist = qd;
        best = Some(c.p_id);
    }
    best
}

/// Whether leadership `getFollowPlayer` counts as a living mother for child gate.
// Haxe: mother != null && mother.isDeleted() == false
pub fn living_follow_leader(leader_p_id: Option<i32>, leader_deleted: bool) -> Option<i32> {
    match leader_p_id {
        Some(id) if id > 0 && !leader_deleted => Some(id),
        _ => None,
    }
}

/// Pure empty-sticky acquire: child-mother first, else AutoFollowPlayer closest.
///
/// Returns `None` when sticky already set, or no valid acquire path.
///
/// 1. If `playerToFollow != null` — no acquire.
/// 2. If `isChildAndHasMother()` — use `getFollowPlayer()` (leadership mother).
/// 3. Else if `AutoFollowPlayer` — use precomputed closest (search radius 20).
// Haxe: AiBase.isMovingToPlayer L8287–8296
pub fn plan_auto_follow_acquire(
    sticky_follow_p_id: i32,
    age: f32,
    leadership_follow_p_id: Option<i32>,
    leadership_follow_deleted: bool,
    auto_follow_player_enabled: bool,
    closest_p_id: Option<i32>,
) -> Option<AutoFollowAcquire> {
    plan_auto_follow_acquire_ex(
        sticky_follow_p_id,
        age,
        leadership_follow_p_id,
        leadership_follow_deleted,
        auto_follow_player_enabled,
        closest_p_id,
        MIN_AGE_TO_EAT,
    )
}

/// Same as [`plan_auto_follow_acquire`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn plan_auto_follow_acquire_ex(
    sticky_follow_p_id: i32,
    age: f32,
    leadership_follow_p_id: Option<i32>,
    leadership_follow_deleted: bool,
    auto_follow_player_enabled: bool,
    closest_p_id: Option<i32>,
    min_age_to_eat: f32,
) -> Option<AutoFollowAcquire> {
    if sticky_follow_p_id > 0 {
        return None;
    }
    let mother = living_follow_leader(leadership_follow_p_id, leadership_follow_deleted);
    if is_child_and_has_mother_ex(age, mother.is_some(), min_age_to_eat) {
        return mother.map(|follow_p_id| AutoFollowAcquire {
            follow_p_id,
            source: AutoFollowAcquireSource::ChildMother,
        });
    }
    if !auto_follow_player_enabled {
        return None;
    }
    closest_p_id
        .filter(|&id| id > 0)
        .map(|follow_p_id| AutoFollowAcquire {
            follow_p_id,
            source: AutoFollowAcquireSource::ClosestPlayer,
        })
}

/// Convenience: plan acquire + compute closest in one pure step for tests/live.
// Haxe: isMovingToPlayer getClosestPlayer(20, followHuman)
pub fn resolve_auto_follow_acquire(
    sticky_follow_p_id: i32,
    age: f32,
    ai_x: i32,
    ai_y: i32,
    ai_p_id: i32,
    leadership_follow_p_id: Option<i32>,
    leadership_follow_deleted: bool,
    auto_follow_player_enabled: bool,
    only_human: bool,
    candidates: &[AutoFollowCandidate],
) -> Option<AutoFollowAcquire> {
    resolve_auto_follow_acquire_ex(
        sticky_follow_p_id,
        age,
        ai_x,
        ai_y,
        ai_p_id,
        leadership_follow_p_id,
        leadership_follow_deleted,
        auto_follow_player_enabled,
        only_human,
        candidates,
        MIN_AGE_TO_EAT,
    )
}

/// Same as [`resolve_auto_follow_acquire`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn resolve_auto_follow_acquire_ex(
    sticky_follow_p_id: i32,
    age: f32,
    ai_x: i32,
    ai_y: i32,
    ai_p_id: i32,
    leadership_follow_p_id: Option<i32>,
    leadership_follow_deleted: bool,
    auto_follow_player_enabled: bool,
    only_human: bool,
    candidates: &[AutoFollowCandidate],
    min_age_to_eat: f32,
) -> Option<AutoFollowAcquire> {
    let closest = if auto_follow_player_enabled
        && sticky_follow_p_id <= 0
        && !is_child_and_has_mother_ex(
            age,
            living_follow_leader(leadership_follow_p_id, leadership_follow_deleted).is_some(),
            min_age_to_eat,
        ) {
        get_closest_player_for_auto_follow(
            ai_x,
            ai_y,
            AUTO_FOLLOW_SEARCH_TILES,
            only_human,
            ai_p_id,
            candidates,
        )
    } else {
        None
    };
    plan_auto_follow_acquire_ex(
        sticky_follow_p_id,
        age,
        leadership_follow_p_id,
        leadership_follow_deleted,
        auto_follow_player_enabled,
        closest,
        min_age_to_eat,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_follow_sticky_clear_ex_live_min_age() {
        let sticky = AiFollowSticky {
            follow_p_id: 7,
            auto_stop_follow: true,
            follow_started_sim_time: 0.0,
        };
        // age 7 > default 6 clear, but live min=5 → clear at >10
        let p = plan_follow_sticky_clear_ex(&sticky, 7.0, 100.0, 5.0);
        assert!(!p.clear_target);
        let p2 = plan_follow_sticky_clear_ex(&sticky, 10.1, 100.0, 5.0);
        assert!(p2.clear_target);
    }

    #[test]
    fn follow_max_tiles_for_context_ex_live_baby_hungry() {
        // age 4 with min 5 + hungry → baby hungry tiles
        let t = follow_max_tiles_for_context_ex(4.0, true, true, false, false, true, 5.0);
        assert_eq!(t, baby_hungry_follow_tiles());
        // age 4 with min 5 + not hungry + mother → still child band under live min
        let t2 = follow_max_tiles_for_context_ex(4.0, false, true, false, false, true, 5.0);
        assert_eq!(t2, child_with_mother_follow_tiles(false));
        // age 4 with min 3 → adult sticky band (loose auto_stop)
        let t3 = follow_max_tiles_for_context_ex(4.0, true, true, false, false, true, 3.0);
        assert_eq!(t3, follow_max_tiles_for_sticky(true));
    }


    #[test]
    fn ordered_follow_timeout_sets_auto_stop() {
        let sticky = AiFollowSticky {
            follow_p_id: 42,
            auto_stop_follow: false,
            follow_started_sim_time: 10.0,
        };
        // 4 min — still ordered
        let p = plan_follow_sticky_clear(&sticky, 20.0, 10.0 + 240.0);
        assert!(!p.set_auto_stop);
        assert!(!p.clear_target);
        // 5 min + epsilon, age young → set auto_stop only (keep target)
        let p = plan_follow_sticky_clear(
            &sticky,
            AUTO_STOP_FOLLOW_CLEAR_AGE - 1.0,
            10.0 + ORDERED_FOLLOW_MAX_SECS + 0.1,
        );
        assert!(p.set_auto_stop);
        assert!(!p.clear_target);
        let mut s = sticky;
        apply_follow_sticky_clear(&mut s, p);
        assert!(s.auto_stop_follow);
        assert_eq!(s.follow_p_id, 42);
        // age > MinAgeToEat*2 with auto_stop → clear target (same-frame if age already high)
        let p2 = plan_follow_sticky_clear(&s, AUTO_STOP_FOLLOW_CLEAR_AGE + 0.1, 400.0);
        assert!(p2.clear_target);
        // timeout + adult age in one plan clears both
        let p3 = plan_follow_sticky_clear(
            &sticky,
            20.0,
            10.0 + ORDERED_FOLLOW_MAX_SECS + 0.1,
        );
        assert!(p3.set_auto_stop);
        assert!(p3.clear_target);
    }

    #[test]
    fn auto_stop_young_keeps_target() {
        let sticky = AiFollowSticky {
            follow_p_id: 7,
            auto_stop_follow: true,
            follow_started_sim_time: 0.0,
        };
        // age 5 ≤ 6 — keep
        let p = plan_follow_sticky_clear(&sticky, 5.0, 100.0);
        assert!(!p.clear_target);
        let p = plan_follow_sticky_clear(&sticky, 6.1, 100.0);
        assert!(p.clear_target);
    }

    #[test]
    fn stand_half_range_matches_haxe_squared_gate() {
        assert_eq!(follow_stand_half_range(1), 1); // 1 < 9
        assert_eq!(follow_stand_half_range(2), 1); // 4 < 9
        assert_eq!(follow_stand_half_range(3), 2); // 9 >= 9
        assert_eq!(follow_stand_half_range(5), 2);
        assert_eq!(follow_stand_half_range(10), 2);
    }

    #[test]
    fn decide_close_enough_vs_walk() {
        let t = FollowTargetSnap {
            p_id: 2,
            x: 10,
            y: 10,
            deleted: false,
        };
        // dist² = 4, max tiles 3 → max_q=9 → close
        let d = decide_follow_walk(2, Some(t), 12, 10, 3, 0);
        assert_eq!(d, FollowWalkDecision::CloseEnough);
        // dist² = 25, max 3 → walk
        let d = decide_follow_walk(2, Some(t), 15, 10, 3, 0);
        match d {
            FollowWalkDecision::WalkTo { goal_x, goal_y } => {
                assert!((goal_x - 10).abs() <= 2);
                assert!((goal_y - 10).abs() <= 2);
            }
            other => panic!("expected WalkTo, got {other:?}"),
        }
    }

    #[test]
    fn decide_deleted_and_no_target() {
        assert_eq!(
            decide_follow_walk(0, None, 0, 0, 5, 0),
            FollowWalkDecision::NoTarget
        );
        let t = FollowTargetSnap {
            p_id: 9,
            x: 0,
            y: 0,
            deleted: true,
        };
        assert_eq!(
            decide_follow_walk(9, Some(t), 0, 0, 5, 0),
            FollowWalkDecision::TargetDeleted
        );
        assert_eq!(
            decide_follow_walk(9, None, 0, 0, 5, 0),
            FollowWalkDecision::TargetDeleted
        );
    }

    #[test]
    fn ally_goto_is_east_of_speaker() {
        assert_eq!(ally_goto_speaker_xy(5, 7), (6, 7));
    }

    #[test]
    fn max_tiles_ordered_vs_loose() {
        assert_eq!(follow_max_tiles_for_sticky(false), 5);
        assert_eq!(follow_max_tiles_for_sticky(true), 10);
    }

    #[test]
    fn max_tiles_context_priority_bands() {
        // Baby hungry wins over mother/wounded/ordered
        assert_eq!(
            follow_max_tiles_for_context(1.0, true, true, true, true, false),
            5
        );
        // Child mother nice vs not
        assert_eq!(
            follow_max_tiles_for_context(1.0, false, true, true, false, true),
            2
        );
        assert_eq!(
            follow_max_tiles_for_context(1.0, false, true, false, false, true),
            4
        );
        // Wounded adult
        assert_eq!(
            follow_max_tiles_for_context(20.0, false, false, false, true, true),
            2
        );
        // Ordered vs loose adult healthy
        assert_eq!(
            follow_max_tiles_for_context(20.0, false, false, false, false, false),
            5
        );
        assert_eq!(
            follow_max_tiles_for_context(20.0, false, false, false, false, true),
            10
        );
    }

    #[test]
    fn say_follow_target_name_age_or_debug() {
        assert!(!should_say_follow_target_name(2.0, false));
        assert!(should_say_follow_target_name(2.0, true));
        assert!(should_say_follow_target_name(3.1, false));
        assert!(should_say_follow_target_name(20.0, false));
    }

    #[test]
    fn sensors_from_sticky() {
        let ordered = AiFollowSticky {
            follow_p_id: 1,
            auto_stop_follow: false,
            follow_started_sim_time: 0.0,
        };
        assert!(ordered_follow_sensor(&ordered));
        assert!(follow_player_sensor(&ordered));
        let loose = AiFollowSticky {
            follow_p_id: 1,
            auto_stop_follow: true,
            follow_started_sim_time: 0.0,
        };
        assert!(!ordered_follow_sensor(&loose));
        assert!(follow_player_sensor(&loose));
    }

    #[test]
    fn truncate_path_cap() {
        let steps: Vec<_> = (0..20).map(|_| (1, 0)).collect();
        let t = truncate_follow_path_steps(&steps, FOLLOW_PATH_STEP_CAP);
        assert_eq!(t.len(), FOLLOW_PATH_STEP_CAP);
    }

    #[test]
    fn follow_walk_holds_only_on_walk() {
        assert!(!follow_walk_holds_tick(FollowWalkDecision::CloseEnough));
        assert!(follow_walk_holds_tick(FollowWalkDecision::WalkTo {
            goal_x: 1,
            goal_y: 2
        }));
    }

    // ── AI-FOLLOW-ACQUIRE ────────────────────────────────────────────────────

    #[test]
    fn closest_player_prefers_nearer_human_within_radius() {
        let cands = [
            AutoFollowCandidate {
                p_id: 2,
                x: 5,
                y: 0,
                is_human: true,
                deleted: false,
            },
            AutoFollowCandidate {
                p_id: 3,
                x: 15,
                y: 0,
                is_human: true,
                deleted: false,
            },
            AutoFollowCandidate {
                p_id: 4,
                x: 1,
                y: 0,
                is_human: false,
                deleted: false,
            },
        ];
        // only_human: nearer human is 2 (dist 5), AI 4 ignored
        assert_eq!(
            get_closest_player_for_auto_follow(0, 0, 20, true, 1, &cands),
            Some(2)
        );
        // include AI: AI at dist 1 wins
        assert_eq!(
            get_closest_player_for_auto_follow(0, 0, 20, false, 1, &cands),
            Some(4)
        );
        // out of range
        assert_eq!(
            get_closest_player_for_auto_follow(0, 0, 3, true, 1, &cands),
            None
        );
    }

    #[test]
    fn closest_player_skips_self_and_deleted() {
        let cands = [
            AutoFollowCandidate {
                p_id: 1,
                x: 0,
                y: 0,
                is_human: true,
                deleted: false,
            },
            AutoFollowCandidate {
                p_id: 2,
                x: 2,
                y: 0,
                is_human: true,
                deleted: true,
            },
            AutoFollowCandidate {
                p_id: 3,
                x: 4,
                y: 0,
                is_human: true,
                deleted: false,
            },
        ];
        assert_eq!(
            get_closest_player_for_auto_follow(0, 0, 20, true, 1, &cands),
            Some(3)
        );
    }

    #[test]
    fn acquire_child_mother_before_closest() {
        // age < MinAgeToEat (3) + living mother → ChildMother, ignore closest
        let a = plan_auto_follow_acquire(0, 2.0, Some(99), false, true, Some(7));
        assert_eq!(
            a,
            Some(AutoFollowAcquire {
                follow_p_id: 99,
                source: AutoFollowAcquireSource::ChildMother
            })
        );
        // deleted mother → fall through to AutoFollow closest
        let a = plan_auto_follow_acquire(0, 2.0, Some(99), true, true, Some(7));
        assert_eq!(
            a,
            Some(AutoFollowAcquire {
                follow_p_id: 7,
                source: AutoFollowAcquireSource::ClosestPlayer
            })
        );
    }

    #[test]
    fn acquire_auto_follow_disabled_returns_none_for_adult() {
        assert!(plan_auto_follow_acquire(0, 20.0, None, false, false, Some(7)).is_none());
        assert_eq!(
            plan_auto_follow_acquire(0, 20.0, None, false, true, Some(7)),
            Some(AutoFollowAcquire {
                follow_p_id: 7,
                source: AutoFollowAcquireSource::ClosestPlayer
            })
        );
    }

    #[test]
    fn acquire_skips_when_sticky_already_set() {
        assert!(plan_auto_follow_acquire(5, 2.0, Some(99), false, true, Some(7)).is_none());
    }

    #[test]
    fn resolve_auto_follow_acquire_end_to_end() {
        let cands = [
            AutoFollowCandidate {
                p_id: 10,
                x: 8,
                y: 0,
                is_human: true,
                deleted: false,
            },
            AutoFollowCandidate {
                p_id: 11,
                x: 3,
                y: 0,
                is_human: true,
                deleted: false,
            },
        ];
        // Adult + AutoFollow on → closest human 11
        let a = resolve_auto_follow_acquire(
            0, 15.0, 0, 0, 1, None, false, true, true, &cands,
        );
        assert_eq!(
            a,
            Some(AutoFollowAcquire {
                follow_p_id: 11,
                source: AutoFollowAcquireSource::ClosestPlayer
            })
        );
        // Child + mother leadership → mother, no closest needed
        let a = resolve_auto_follow_acquire(
            0,
            1.5,
            0,
            0,
            1,
            Some(50),
            false,
            true,
            true,
            &cands,
        );
        assert_eq!(
            a,
            Some(AutoFollowAcquire {
                follow_p_id: 50,
                source: AutoFollowAcquireSource::ChildMother
            })
        );
        // Default AutoFollow off + adult → none
        let a = resolve_auto_follow_acquire(
            0,
            15.0,
            0,
            0,
            1,
            None,
            false,
            AUTO_FOLLOW_PLAYER_DEFAULT,
            true,
            &cands,
        );
        assert!(a.is_none());
    }
}

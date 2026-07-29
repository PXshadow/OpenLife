//! Multi-use object semantics (Haxe `TransitionHelper` numberOfUses).
//!
//! Port anchors:
//! - `DoChangeNumberOfUsesOnActor` / `DoChangeNumberOfUsesOnActorManual`
//! - `DoChangeNumberOfUsesOnTarget`
//! - reverse-use max checks + max-use transition selection
//! - loved-food bare-hand extra (TH-MULTI-POLISH)
//!
//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** (`loved_food_settings`)

/// Outcome for the ground target after a USE transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetUsesOutcome {
    /// Non multi-use (or cleared to empty): place as simple base id.
    Simple,
    /// Multi-use helper with `uses` remaining (`uses >= 1`).
    Uses(i32),
    /// Uses hit 0 without a last-use id transform — clear the tile.
    Cleared,
}

/// Outcome for the held actor after a USE transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorUsesOutcome {
    /// Held object base id (may change on deplete via last-use tool table).
    pub held_id: i32,
    /// Remaining uses (0 = N/A / single-use).
    pub held_uses: i32,
}

/// Haxe: refuse reverse-use actor when already at max uses on the *new* actor type.
#[inline]
pub fn reverse_actor_exceeds_max(held_uses: i32, new_actor_num_uses: i32) -> bool {
    new_actor_num_uses >= 2 && held_uses >= new_actor_num_uses
}

/// Haxe: reverse-use target would push uses past max on the *new* target type.
#[inline]
pub fn reverse_target_exceeds_max(target_uses: i32, new_target_num_uses: i32) -> bool {
    new_target_num_uses >= 2 && target_uses >= new_target_num_uses
}

/// Clamp target uses that somehow exceed `num_uses` (Haxe warning + clamp).
#[inline]
pub fn clamp_uses(uses: i32, num_uses: i32) -> i32 {
    if num_uses < 2 {
        return uses.max(0);
    }
    if uses > num_uses {
        num_uses
    } else {
        uses.max(0)
    }
}

/// Effective uses for a multi-use tile when the helper is missing.
///
/// Reverse-add defaults to 1 (empty pile building); harvest defaults to full.
#[inline]
pub fn effective_target_uses(uses_before: i32, num_uses: i32, reverse_use_target: bool) -> i32 {
    if uses_before > 0 {
        clamp_uses(uses_before, num_uses)
    } else if reverse_use_target {
        1
    } else if num_uses > 1 {
        num_uses
    } else {
        0
    }
}

/// Haxe `DoChangeNumberOfUsesOnTarget` core (pure).
///
/// `num_uses_before` is the *old* target objectData.numUses (for reset suppression
/// when mining shallow pits keep the same numUses across id change).
///
/// `allow_reset_on_id_change`: Haxe `resetNumberOfUses` — clothing multi-use starts
/// false (`isClothing && numUses >= 2`); also forced false when numUses equal.
pub fn change_number_of_uses_on_target(
    target_before: i32,
    target_after: i32,
    uses_before: i32,
    num_uses_before: i32,
    num_uses_after: i32,
    reverse_use_target: bool,
    no_use_target: bool,
    from_transition: bool,
    allow_reset_on_id_change: bool,
) -> TargetUsesOutcome {
    if target_after == 0 {
        return TargetUsesOutcome::Cleared;
    }
    if num_uses_after < 2 {
        return TargetUsesOutcome::Simple;
    }

    // Bare-hand swap / non-transition placement: keep existing uses or full.
    if !from_transition {
        let u = if uses_before > 0 {
            clamp_uses(uses_before, num_uses_after)
        } else {
            num_uses_after
        };
        return TargetUsesOutcome::Uses(u);
    }

    // Haxe: `if (transition.noUseTarget == false) DoChangeNumberOfUsesOnTarget`
    // When skipped, id is already transformed; keep uses when possible.
    if no_use_target {
        let u = if uses_before > 0 {
            clamp_uses(uses_before, num_uses_after)
        } else {
            num_uses_after
        };
        return TargetUsesOutcome::Uses(u);
    }

    // Haxe: `if (transition.targetNumberOfUses >= 0)` force-set (handled by caller
    // via `force_target_number_of_uses` before this, or pass no_use + pre-set).

    let id_changed = target_before != target_after;
    // Haxe: if oldObjData.numUses == objectData.numUses → resetNumberOfUses = false
    let mut reset_number_of_uses = allow_reset_on_id_change;
    if num_uses_before == num_uses_after && num_uses_before >= 2 {
        reset_number_of_uses = false;
    }

    if id_changed && reset_number_of_uses {
        // New pile/bucket (reverse) starts at 1; deposit/full harvest starts at max.
        let start = if reverse_use_target { 1 } else { num_uses_after };
        return TargetUsesOutcome::Uses(start);
    }

    // Same id, or id changed but same numUses / clothing (preserve + adjust).
    let cur = effective_target_uses(uses_before, num_uses_after, reverse_use_target);

    if reverse_use_target {
        // Haxe: if (obj.numberOfUses > objectData.numUses - 1) return;
        if cur > num_uses_after - 1 {
            return TargetUsesOutcome::Uses(cur.min(num_uses_after));
        }
        return TargetUsesOutcome::Uses((cur + 1).min(num_uses_after));
    }

    let next = cur - 1;
    if next <= 0 {
        TargetUsesOutcome::Cleared
    } else {
        TargetUsesOutcome::Uses(next)
    }
}

/// Haxe `transition.targetNumberOfUses >= 0` force-set (clamped to numUses).
#[inline]
pub fn force_target_number_of_uses(target_number_of_uses: i32, num_uses: i32) -> Option<i32> {
    if target_number_of_uses < 0 || num_uses < 2 {
        return None;
    }
    Some(target_number_of_uses.min(num_uses).max(0))
}

/// Haxe `transition.switchNumberOfUses` — swap held ↔ target use counts.
#[inline]
pub fn switch_number_of_uses(held_uses: i32, target_uses: i32) -> (i32, i32) {
    (target_uses, held_uses)
}

/// Haxe `actorMinUseFraction == 1` — refuse when multi-use actor is not full.
#[inline]
pub fn actor_must_be_full_refuse(
    actor_min_use_fraction: f32,
    held_uses: i32,
    held_num_uses: i32,
) -> bool {
    actor_min_use_fraction >= 1.0 && held_num_uses > 1 && held_uses < held_num_uses
}

/// Haxe `targetMinUseFraction == 1 && !reverseUseTarget` — refuse when target not full.
#[inline]
pub fn target_must_be_full_refuse(
    target_min_use_fraction: f32,
    reverse_use_target: bool,
    target_uses: i32,
    target_num_uses: i32,
) -> bool {
    !reverse_use_target
        && target_min_use_fraction >= 1.0
        && target_num_uses > 1
        && target_uses < target_num_uses
}

/// Haxe tool durability / probabilistic use: skip decrement when roll fails.
///
/// Actor: `useChance > 0 && random > useChance` → skip.  
/// Target non-reverse: decrement only if `useChance <= 0 || random < useChance`.
#[inline]
pub fn should_skip_use_decrement(use_chance: f32, rng01: f32) -> bool {
    use_chance > 0.0 && rng01 > use_chance
}

/// Haxe tool last-use chain after actor uses hit 0 (or import rewrite lookup):
/// `(id, -1)` last-use-actor, then `(id, -1)` non-LA, then `(id, target)` LA.
///
/// Each argument is the **new_actor_id** of the matching transition, if any.
/// Haxe TODO TransitionHelper L1565 (EMPTY+Cold Bowl): no extra filter here —
/// such edges are simply absent from `(id,-1)` tool keys / skipped at import.
// Haxe: TransitionHelper.DoChangeNumberOfUsesOnActorManual (~1567–1590)
// Haxe: TransitionImporter.changeToolTransitions tool lookup (~284–290)
#[inline]
pub fn pick_tool_last_use_new_actor(
    last_use_on_empty: Option<i32>,
    normal_on_empty: Option<i32>,
    last_use_on_target: Option<i32>,
) -> Option<i32> {
    last_use_on_empty
        .or(normal_on_empty)
        .or(last_use_on_target)
}

/// Outcome of Haxe `DoChangeNumberOfUsesOnActorManual` on the **eat** path
/// (`idHasChanged=false`, `reverseUse=false`, `targetId=-1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EatActorUsesOutcome {
    /// useChance skipped decrement — keep held as-is.
    Unchanged { held_id: i32, held_uses: i32 },
    /// Multi-use still has remaining uses.
    Keep { held_id: i32, held_uses: i32 },
    /// Uses hit 0; tool last-use rewrote held id (Haxe keeps `numberOfUses` at 0).
    Transformed { held_id: i32 },
    /// Uses hit 0 and no tool row → clear held (eat returns false).
    Clear,
}

/// Pure Haxe eat-path actor uses after food is already applied.
///
/// `tool_new_actor` is [`pick_tool_last_use_new_actor`] for `(held_id, -1)`.
/// When uses would go to 0 and `tool_new_actor` is `Some(0)`, treat as [`EatActorUsesOutcome::Clear`].
// Haxe: TransitionHelper.DoChangeNumberOfUsesOnActorManual (eat callers pass reverse=false, idChanged=false)
pub fn eat_actor_after_use(
    held_id: i32,
    uses_before: i32,
    num_uses: i32,
    use_chance: f32,
    rng01: f32,
    tool_new_actor: Option<i32>,
) -> EatActorUsesOutcome {
    if held_id == 0 {
        return EatActorUsesOutcome::Clear;
    }
    // Effective uses before this bite (multi-use full when unset; single-use = one bite).
    let cur = if uses_before > 0 {
        if num_uses >= 2 {
            clamp_uses(uses_before, num_uses)
        } else {
            uses_before
        }
    } else if num_uses >= 2 {
        num_uses
    } else {
        1
    };
    if should_skip_use_decrement(use_chance, rng01) {
        return EatActorUsesOutcome::Unchanged {
            held_id,
            held_uses: if num_uses >= 2 { cur } else { uses_before.max(0) },
        };
    }
    let next = cur - 1;
    if next > 0 {
        return EatActorUsesOutcome::Keep {
            held_id,
            held_uses: next,
        };
    }
    match tool_new_actor {
        Some(0) | None => EatActorUsesOutcome::Clear,
        Some(new_id) => EatActorUsesOutcome::Transformed { held_id: new_id },
    }
}

/// Haxe `DoChangeNumberOfUsesOnActorManual` core (pure, without tool last-use lookup).
///
/// When uses hit 0, returns `held_id` still at `actor_after` with `held_uses=0` so
/// the caller can run tool / last-use actor transitions (id may change).
pub fn change_number_of_uses_on_actor(
    actor_before: i32,
    actor_after: i32,
    uses_before: i32,
    num_uses_after: i32,
    reverse_use_actor: bool,
    no_use_actor: bool,
) -> ActorUsesOutcome {
    if actor_after == 0 {
        return ActorUsesOutcome {
            held_id: 0,
            held_uses: 0,
        };
    }

    if no_use_actor {
        // Preserve uses when possible; single-use → 0.
        if num_uses_after < 2 {
            return ActorUsesOutcome {
                held_id: actor_after,
                held_uses: 0,
            };
        }
        let u = if actor_before == actor_after && uses_before > 0 {
            clamp_uses(uses_before, num_uses_after)
        } else if uses_before > 0 {
            clamp_uses(uses_before, num_uses_after)
        } else {
            num_uses_after
        };
        return ActorUsesOutcome {
            held_id: actor_after,
            held_uses: u,
        };
    }

    let id_changed = actor_before != actor_after;

    if id_changed {
        if reverse_use_actor {
            // Berry into empty bowl from tree → start at 1.
            return ActorUsesOutcome {
                held_id: actor_after,
                held_uses: if num_uses_after >= 2 { 1 } else { 0 },
            };
        }
        if num_uses_after < 2 {
            return ActorUsesOutcome {
                held_id: actor_after,
                held_uses: 0,
            };
        }
        // Cooked pie / full multi-use result.
        return ActorUsesOutcome {
            held_id: actor_after,
            held_uses: num_uses_after,
        };
    }

    // Same held id.
    if num_uses_after < 2 {
        return ActorUsesOutcome {
            held_id: actor_after,
            held_uses: 0,
        };
    }

    let cur = if uses_before > 0 {
        clamp_uses(uses_before, num_uses_after)
    } else {
        num_uses_after
    };

    if reverse_use_actor {
        return ActorUsesOutcome {
            held_id: actor_after,
            held_uses: (cur + 1).min(num_uses_after),
        };
    }

    let next = cur - 1;
    ActorUsesOutcome {
        held_id: actor_after,
        held_uses: next.max(0),
    }
}

/// Prefer last-use transition table when multi-use is nearly exhausted.
///
/// Haxe: target `isLastUse()` when `numberOfUses <= 1` on multi-use; actor similar
/// for lastUseActor paths. Rust also honors explicit force/prefer flags.
#[inline]
pub fn prefer_last_use_table(
    force_or_global: bool,
    target_uses: i32,
    target_num_uses: i32,
    held_uses: i32,
    held_num_uses: i32,
) -> bool {
    if force_or_global {
        return true;
    }
    if target_num_uses >= 2 && target_uses == 1 {
        return true;
    }
    if held_num_uses >= 2 && held_uses == 1 {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// TH-MULTI-POLISH: loved-food bare-hand extra
// Haxe: `TransitionHelper.DoChangeNumberOfUsesOnTarget` + `Biome.getLovedPlants`
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.LovedFoodUseChance` (base chance of normal harvest, not extra).
pub const LOVED_FOOD_USE_CHANCE: f32 = 0.5;

/// Haxe `PersonColor` race ids (ObjectData.person field).
pub const PERSON_BLACK: i32 = 1;
pub const PERSON_BROWN: i32 = 3;
pub const PERSON_WHITE: i32 = 4;
pub const PERSON_GINGER: i32 = 6;

/// Haxe biome ids used by loved plants (BiomeTag).
pub const BIOME_GREY: i32 = 3;
pub const BIOME_SNOW: i32 = 4;
pub const BIOME_DESERT: i32 = 5;
pub const BIOME_JUNGLE: i32 = 6;

/// Haxe `Biome.GetLovedBiomeByPlayer` via person color.
#[inline]
pub fn loved_biome_for_person_color(person_color: i32) -> Option<i32> {
    match person_color {
        PERSON_GINGER => Some(BIOME_SNOW),
        PERSON_WHITE => Some(BIOME_GREY),
        PERSON_BROWN => Some(BIOME_JUNGLE),
        PERSON_BLACK => Some(BIOME_DESERT),
        _ => None,
    }
}

/// Haxe `Biome.getLovedPlants(biomeTag)`.
#[inline]
pub fn loved_plants_for_biome(biome_tag: i32) -> &'static [i32] {
    match biome_tag {
        BIOME_DESERT => &[763],  // Fruiting Barrel Cactus
        BIOME_JUNGLE => &[2142], // Banana Plant
        BIOME_GREY => &[4251],   // Wild Garlic (on ground)
        BIOME_SNOW => &[39],     // Dug Wild Carrot
        _ => &[],
    }
}

/// Haxe `player.getLovedPlants()` from person race color.
#[inline]
pub fn loved_plants_for_person_color(person_color: i32) -> &'static [i32] {
    match loved_biome_for_person_color(person_color) {
        Some(b) => loved_plants_for_biome(b),
        None => &[],
    }
}

/// True when `target_id` is a loved plant for this person color.
#[inline]
pub fn is_loved_plant_target(person_color: i32, target_id: i32) -> bool {
    loved_plants_for_person_color(person_color).contains(&target_id)
}

/// Haxe effective loved-food chance: `LovedFoodUseChance + obj.hits / 10`.
#[inline]
pub fn loved_food_effective_chance(base_chance: f32, object_hits: f32) -> f32 {
    base_chance + object_hits / 10.0
}

/// Haxe: bare-hand non-reverse on loved plant — `rand > useChance` → extra hit.
///
/// Returns true when the harvest should **not** consume a plant use (player still
/// receives the actor/new food from the transition).
#[inline]
pub fn loved_food_extra_hit(effective_chance: f32, rng01: f32) -> bool {
    rng01 > effective_chance
}

/// Pure outcome for loved-food extra on the target tile after transform.
///
/// Haxe after `target.id = newTargetID`:
/// - multi-use (`numUses > 1`): keep transformed id, do not decrement uses, hits++
/// - single-use: restore `obj.id = transition.targetID` (original plant), hits++
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LovedFoodExtraTarget {
    /// Keep `target_after`, skip use decrement (`no_use_target` path).
    KeepTransformedNoUse,
    /// Restore original plant id; skip normal place-after-use.
    RestoreOriginal,
}

/// Decide target placement when loved-food extra fires.
///
/// `num_uses_after` is Haxe `objectData.numUses` of the **already-transformed** target.
#[inline]
pub fn loved_food_extra_target_outcome(num_uses_after: i32) -> LovedFoodExtraTarget {
    if num_uses_after > 1 {
        LovedFoodExtraTarget::KeepTransformedNoUse
    } else {
        LovedFoodExtraTarget::RestoreOriginal
    }
}

/// Gate: Haxe `reverseUse == false && transition.actorID == 0`.
#[inline]
pub fn loved_food_bare_hand_gate(actor_id: i32, reverse_use_target: bool) -> bool {
    actor_id == 0 && !reverse_use_target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pile_starts_at_one_on_reverse_new_id() {
        let o = change_number_of_uses_on_target(
            33, 661, 0, 0, 9, true, false, true, true,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(1));
    }

    #[test]
    fn pile_increments_same_id_reverse() {
        let o = change_number_of_uses_on_target(
            661, 661, 2, 9, 9, true, false, true, true,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(3));
    }

    #[test]
    fn pile_does_not_exceed_max_on_reverse() {
        let o = change_number_of_uses_on_target(
            661, 661, 9, 9, 9, true, false, true, true,
        );
        // cur=9 > numUses-1=8 → no change
        assert_eq!(o, TargetUsesOutcome::Uses(9));
    }

    #[test]
    fn harvest_decrements_then_clears() {
        let o = change_number_of_uses_on_target(
            50, 50, 2, 3, 3, false, false, true, true,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(1));
        let o2 = change_number_of_uses_on_target(
            50, 50, 1, 3, 3, false, false, true, true,
        );
        assert_eq!(o2, TargetUsesOutcome::Cleared);
    }

    #[test]
    fn same_num_uses_id_change_preserves_and_decrements() {
        // Mining pit style: id changes, numUses equal → no reset, then -=1
        let o = change_number_of_uses_on_target(
            100, 101, 5, 8, 8, false, false, true, true,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(4));
    }

    #[test]
    fn no_use_target_keeps_uses() {
        let o = change_number_of_uses_on_target(
            661, 661, 4, 9, 9, false, true, true, true,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(4));
    }

    #[test]
    fn clothing_suppress_reset_on_id_change() {
        // Haxe: clothing multi-use → resetNumberOfUses=false → preserve + decrement
        let o = change_number_of_uses_on_target(
            200, 201, 4, 5, 6, false, false, true, false,
        );
        assert_eq!(o, TargetUsesOutcome::Uses(3));
    }

    #[test]
    fn switch_uses_swaps() {
        assert_eq!(switch_number_of_uses(3, 7), (7, 3));
    }

    #[test]
    fn min_use_fraction_gates() {
        assert!(actor_must_be_full_refuse(1.0, 2, 5));
        assert!(!actor_must_be_full_refuse(1.0, 5, 5));
        assert!(!actor_must_be_full_refuse(0.0, 2, 5));
        assert!(target_must_be_full_refuse(1.0, false, 2, 5));
        assert!(!target_must_be_full_refuse(1.0, true, 2, 5)); // reverse exempt
        assert!(!target_must_be_full_refuse(1.0, false, 5, 5));
    }

    #[test]
    fn use_chance_skip() {
        assert!(should_skip_use_decrement(0.5, 0.9));
        assert!(!should_skip_use_decrement(0.5, 0.1));
        assert!(!should_skip_use_decrement(0.0, 0.9));
        assert!(!should_skip_use_decrement(1.0, 0.5));
    }

    #[test]
    fn force_target_uses() {
        assert_eq!(force_target_number_of_uses(2, 5), Some(2));
        assert_eq!(force_target_number_of_uses(9, 5), Some(5));
        assert_eq!(force_target_number_of_uses(-1, 5), None);
    }

    #[test]
    fn actor_reverse_new_starts_at_one() {
        let o = change_number_of_uses_on_actor(0, 253, 0, 5, true, false);
        assert_eq!(o.held_id, 253);
        assert_eq!(o.held_uses, 1);
    }

    #[test]
    fn actor_id_change_full_uses() {
        let o = change_number_of_uses_on_actor(0, 253, 0, 5, false, false);
        assert_eq!(o.held_uses, 5);
    }

    #[test]
    fn actor_same_id_decrements() {
        let o = change_number_of_uses_on_actor(253, 253, 3, 5, false, false);
        assert_eq!(o.held_uses, 2);
    }

    #[test]
    fn reverse_exceeds_helpers() {
        assert!(reverse_target_exceeds_max(9, 9));
        assert!(!reverse_target_exceeds_max(8, 9));
        assert!(reverse_actor_exceeds_max(5, 5));
        assert!(!reverse_actor_exceeds_max(4, 5));
    }

    #[test]
    fn prefer_last_when_one_use_left() {
        assert!(prefer_last_use_table(false, 1, 3, 0, 0));
        assert!(prefer_last_use_table(false, 0, 0, 1, 4));
        assert!(!prefer_last_use_table(false, 2, 3, 2, 4));
        assert!(prefer_last_use_table(true, 2, 3, 2, 4));
    }

    #[test]
    fn actor_same_id_to_zero() {
        let o = change_number_of_uses_on_actor(253, 253, 1, 5, false, false);
        assert_eq!(o.held_uses, 0);
        assert_eq!(o.held_id, 253);
    }

    #[test]
    fn pick_tool_last_use_prefers_la_then_non_la() {
        assert_eq!(
            pick_tool_last_use_new_actor(Some(235), Some(1251), None),
            Some(235)
        );
        assert_eq!(
            pick_tool_last_use_new_actor(None, Some(382), None),
            Some(382)
        );
        assert_eq!(
            pick_tool_last_use_new_actor(None, None, Some(99)),
            Some(99)
        );
        assert_eq!(pick_tool_last_use_new_actor(None, None, None), None);
    }

    /// Bowl of Stew 1251 numUses=2: uses 2→1 keep; 1→0 LA → Clay Bowl 235.
    // Haxe: GlobalPlayerInstance eat + DoChangeNumberOfUsesOnActorManual
    #[test]
    fn eat_bowl_of_stew_to_clay_bowl() {
        // uses=2 → still 1251 uses=1
        assert_eq!(
            eat_actor_after_use(1251, 2, 2, 0.0, 0.0, Some(235)),
            EatActorUsesOutcome::Keep {
                held_id: 1251,
                held_uses: 1
            }
        );
        // uses=1 → tool LA 1251+-1 → 235, uses stay 0
        assert_eq!(
            eat_actor_after_use(1251, 1, 2, 0.0, 0.0, Some(235)),
            EatActorUsesOutcome::Transformed { held_id: 235 }
        );
        // no tool row → clear
        assert_eq!(
            eat_actor_after_use(1251, 1, 2, 0.0, 0.0, None),
            EatActorUsesOutcome::Clear
        );
        // single-use food with banana peel tool
        assert_eq!(
            eat_actor_after_use(2143, 0, 0, 0.0, 0.0, Some(2144)),
            EatActorUsesOutcome::Transformed { held_id: 2144 }
        );
        // single-use no tool → clear
        assert_eq!(
            eat_actor_after_use(33, 0, 0, 0.0, 0.0, None),
            EatActorUsesOutcome::Clear
        );
    }

    #[test]
    fn actor_reverse_same_id_increments() {
        let o = change_number_of_uses_on_actor(253, 253, 2, 5, true, false);
        assert_eq!(o.held_uses, 3);
    }

    #[test]
    fn loved_plants_by_race() {
        assert_eq!(loved_plants_for_person_color(PERSON_BROWN), &[2142]);
        assert_eq!(loved_plants_for_person_color(PERSON_BLACK), &[763]);
        assert_eq!(loved_plants_for_person_color(PERSON_WHITE), &[4251]);
        assert_eq!(loved_plants_for_person_color(PERSON_GINGER), &[39]);
        assert!(loved_plants_for_person_color(0).is_empty());
        assert!(is_loved_plant_target(PERSON_BROWN, 2142));
        assert!(!is_loved_plant_target(PERSON_BROWN, 39));
    }

    #[test]
    fn loved_food_extra_roll_and_target() {
        assert!(loved_food_bare_hand_gate(0, false));
        assert!(!loved_food_bare_hand_gate(1, false));
        assert!(!loved_food_bare_hand_gate(0, true));

        let ch = loved_food_effective_chance(LOVED_FOOD_USE_CHANCE, 0.0);
        assert!((ch - 0.5).abs() < 1e-5);
        let ch2 = loved_food_effective_chance(LOVED_FOOD_USE_CHANCE, 5.0);
        assert!((ch2 - 1.0).abs() < 1e-5);

        // rand > chance → extra
        assert!(loved_food_extra_hit(0.5, 0.9));
        assert!(!loved_food_extra_hit(0.5, 0.1));
        // hits raise chance so extras become rarer
        assert!(!loved_food_extra_hit(1.0, 0.9));

        assert_eq!(
            loved_food_extra_target_outcome(5),
            LovedFoodExtraTarget::KeepTransformedNoUse
        );
        assert_eq!(
            loved_food_extra_target_outcome(1),
            LovedFoodExtraTarget::RestoreOriginal
        );
        assert_eq!(
            loved_food_extra_target_outcome(0),
            LovedFoodExtraTarget::RestoreOriginal
        );
    }
}

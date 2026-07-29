//! Alternative transition outcome + fortification fail path (**TH-ALT-OUTCOME**).
//!
//! Haxe `TransitionHelper.doTransitionIfPossibleHelper` L1260–1306:
//! - Prefer `transition.alternativeTransitionOutcome`, else new-target object list
//! - Gate: targetID ≠ 884, !allowForOwner, (outcomes non-empty ∨ fortified)
//! - Roll: `rng01 + hits / AlternativeOutcomePercentIncreasePerHit`
//! - Fail (`roll < 1`): hits+=1, place bonus/fort material, keep tile (no main transform)
//! - Success: hits −= AlternativeOutcomeHitsDecreaseOnSuccess, continue normal transition
//
// Haxe: TransitionHelper L1260–1306 alternativeTransitionOutcome
// Haxe: ServerSettings.AlternativeOutcomePercentIncreasePerHit / HitsDecreaseOnSucess

use ol_content::ContentDb;

/// Haxe `ServerSettings.AlternativeOutcomePercentIncreasePerHit` (default 10).
pub const ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT: f32 = 10.0;
/// Haxe `ServerSettings.AlternativeOutcomeHitsDecreaseOnSucess` (default 5; Haxe spelling).
pub const ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS: f32 = 5.0;
/// Haxe Stone Floor 884 — bugfix excluded from alt-outcome gate.
pub const STONE_FLOOR_BUGFIX_ID: i32 = 884;

/// Fortified when hungry-work cost > 0 and tile hits are negative (fort layers).
// Haxe: TransitionHelper L1179 isFortified = hungryWorkCost > 0 && target.hits < -0.1
#[inline]
pub fn is_fortified_hits(hungry_work_cost: f32, target_hits: f32) -> bool {
    hungry_work_cost > 0.0 && target_hits < -0.1
}

/// Whether the alt-outcome / fortification block applies.
// Haxe: L1267 transition.targetID != 884 && !allowForOwner && (outcomes|fortified)
#[inline]
pub fn alt_outcome_gate_applies(
    transition_target_id: i32,
    allow_for_owner: bool,
    outcomes_non_empty: bool,
    is_fortified: bool,
) -> bool {
    transition_target_id != STONE_FLOOR_BUGFIX_ID
        && !allow_for_owner
        && (outcomes_non_empty || is_fortified)
}

/// Effective roll: `rng01 + hits / percent_per_hit`.
// Haxe: L1269–1271
#[inline]
pub fn alt_outcome_effective_roll(
    rng01: f32,
    target_hits: f32,
    percent_increase_per_hit: f32,
) -> f32 {
    let div = if percent_increase_per_hit.is_finite() && percent_increase_per_hit > 0.0 {
        percent_increase_per_hit
    } else {
        ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT
    };
    let hits = if target_hits.is_finite() {
        target_hits
    } else {
        0.0
    };
    let r = if rng01.is_finite() {
        rng01.clamp(0.0, 1.0)
    } else {
        0.0
    };
    r + hits / div
}

/// Pick uniform index in `0..=len-1` (Haxe `calculateRandomInt(len-1)`).
// Haxe: WorldMap.randomInt(x) = floor(random * (x+1))
#[inline]
pub fn pick_outcome_index(len: usize, rng01: f32) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let r = if rng01.is_finite() {
        rng01.clamp(0.0, 0.999_999)
    } else {
        0.0
    };
    // maxInt = len-1 → floor(r * len)
    Some(((r * len as f32).floor() as usize).min(len - 1))
}

/// Fortification material drop chance: `0.8 / fortificationValue`.
// Haxe: L1284 dropChance = 0.8 / fortificationMaterial.fortificationValue
#[inline]
pub fn fortification_drop_chance(fortification_value: f32) -> f32 {
    let v = if fortification_value.is_finite() && fortification_value > 0.0 {
        fortification_value
    } else {
        1.0
    };
    0.8 / v
}

/// Pure plan for one alt-outcome roll.
#[derive(Debug, Clone, PartialEq)]
pub enum AltOutcomePlan {
    /// Skip block — continue normal transition unchanged.
    Skip,
    /// Fail roll: keep target id; stamp hits; maybe place extra object.
    TryAgain {
        hits_after: f32,
        /// New countObj after optional fort drop (None = leave unchanged).
        count_obj_after: Option<f32>,
        /// Object id to PlaceObjectById near tile (0 / None = no place).
        place_id: Option<i32>,
        say_fortification: bool,
    },
    /// Success roll: reduce hits, continue main transition.
    Proceed { hits_after: f32 },
}

/// Resolve outcomes: transition list wins if non-empty, else object list for new_target
/// (and its dummy parent base id).
// Haxe: L1260–1261
pub fn resolve_alternative_outcomes<'a>(
    content: &'a ContentDb,
    actor_id: i32,
    target_id: i32,
    new_target_id: i32,
) -> &'a [i32] {
    content.alternative_outcomes_for(actor_id, target_id, new_target_id)
}

/// Evaluate alt-outcome / fortification path (pure).
///
/// `rng_success` — Haxe `calculateRandomFloat()` for the success gate.
/// `rng_place` — second float for fort drop **or** outcome index pick.
// Haxe: TransitionHelper L1260–1306
pub fn evaluate_alternative_outcome(
    transition_target_id: i32,
    allow_for_owner: bool,
    is_fortified: bool,
    outcomes: &[i32],
    target_hits: f32,
    count_obj: f32,
    fortification_obj_id: i32,
    fortification_value: f32,
    percent_increase_per_hit: f32,
    hits_decrease_on_success: f32,
    rng_success: f32,
    rng_place: f32,
) -> AltOutcomePlan {
    if !alt_outcome_gate_applies(
        transition_target_id,
        allow_for_owner,
        !outcomes.is_empty(),
        is_fortified,
    ) {
        return AltOutcomePlan::Skip;
    }
    let roll = alt_outcome_effective_roll(rng_success, target_hits, percent_increase_per_hit);
    if roll < 1.0 {
        let hits_after = target_hits + 1.0;
        // Fort material drop when countObj>0 and fortificationObjId>0
        if count_obj > 0.0 && fortification_obj_id > 0 {
            let chance = fortification_drop_chance(fortification_value);
            let place = if rng_place < chance {
                Some(fortification_obj_id)
            } else {
                None
            };
            let count_after = if place.is_some() {
                Some((count_obj - 1.0).max(0.0))
            } else {
                None
            };
            return AltOutcomePlan::TryAgain {
                hits_after,
                count_obj_after: count_after,
                place_id: place,
                say_fortification: is_fortified,
            };
        }
        // Else: random alternative outcome when list non-empty and no fort id
        // Haxe: L1291–1298 else branch when !(countObj>0 && fortId>0)
        // and inner: outcomes.length > 0 && fortificationObjId < 1
        let place_id = if !outcomes.is_empty() && fortification_obj_id < 1 {
            pick_outcome_index(outcomes.len(), rng_place)
                .map(|i| outcomes[i])
                .filter(|&id| id > 0)
        } else {
            None
        };
        AltOutcomePlan::TryAgain {
            hits_after,
            count_obj_after: None,
            place_id,
            say_fortification: is_fortified,
        }
    } else {
        let dec = if hits_decrease_on_success.is_finite() {
            hits_decrease_on_success
        } else {
            ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS
        };
        AltOutcomePlan::Proceed {
            hits_after: target_hits - dec,
        }
    }
}

/// Convenience: content-backed fortification id/value for a tile object.
#[inline]
pub fn fortification_of(content: &ContentDb, object_id: i32) -> (i32, f32) {
    let base = content.resolve_base_id(object_id);
    let fort_id = content
        .fortification_obj_id
        .get(&base)
        .or_else(|| content.fortification_obj_id.get(&object_id))
        .copied()
        .unwrap_or(0);
    let fort_val = if fort_id > 0 {
        content
            .fortification_value
            .get(&fort_id)
            .copied()
            .unwrap_or(1.0)
    } else {
        1.0
    };
    (fort_id, fort_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::apply_default_alternative_outcome_patches;

    #[test]
    fn gate_skips_stone_floor_and_owner() {
        assert!(!alt_outcome_gate_applies(884, false, true, false));
        assert!(!alt_outcome_gate_applies(340, true, true, false));
        assert!(alt_outcome_gate_applies(340, false, true, false));
        assert!(alt_outcome_gate_applies(550, false, false, true));
        assert!(!alt_outcome_gate_applies(550, false, false, false));
    }

    #[test]
    fn roll_ramps_with_hits() {
        // 0 hits: roll in [0,1) → always try-again
        assert!(alt_outcome_effective_roll(0.99, 0.0, 10.0) < 1.0);
        // 10 hits: always ≥ 1
        assert!(alt_outcome_effective_roll(0.0, 10.0, 10.0) >= 1.0);
        // 5 hits + 0.4 rng → 0.9 fail; +0.6 → 1.1 success
        assert!(alt_outcome_effective_roll(0.4, 5.0, 10.0) < 1.0);
        assert!(alt_outcome_effective_roll(0.6, 5.0, 10.0) >= 1.0);
    }

    #[test]
    fn try_again_places_outcome_and_increments_hits() {
        let plan = evaluate_alternative_outcome(
            340,
            false,
            false,
            &[344, 345],
            0.0,
            0.0,
            0,
            1.0,
            10.0,
            5.0,
            0.5, // fail
            0.0, // first outcome 344
        );
        match plan {
            AltOutcomePlan::TryAgain {
                hits_after,
                place_id,
                say_fortification,
                ..
            } => {
                assert!((hits_after - 1.0).abs() < 1e-5);
                assert_eq!(place_id, Some(344));
                assert!(!say_fortification);
            }
            other => panic!("expected TryAgain, got {other:?}"),
        }
    }

    #[test]
    fn try_again_skips_place_when_outcome_zero() {
        let plan = evaluate_alternative_outcome(
            3961,
            false,
            false,
            &[0, 33],
            2.0,
            0.0,
            0,
            1.0,
            10.0,
            5.0,
            0.1, // fail (2/10 + 0.1 = 0.3)
            0.0, // index 0 → 0
        );
        match plan {
            AltOutcomePlan::TryAgain { place_id, .. } => assert!(place_id.is_none()),
            other => panic!("expected TryAgain, got {other:?}"),
        }
    }

    #[test]
    fn proceed_decrements_hits() {
        let plan = evaluate_alternative_outcome(
            340,
            false,
            false,
            &[344],
            10.0,
            0.0,
            0,
            1.0,
            10.0,
            5.0,
            0.0, // 0+1.0 = 1.0 not < 1 → proceed
            0.5,
        );
        match plan {
            AltOutcomePlan::Proceed { hits_after } => {
                assert!((hits_after - 5.0).abs() < 1e-5);
            }
            other => panic!("expected Proceed, got {other:?}"),
        }
    }

    #[test]
    fn fort_drop_uses_count_and_value() {
        // chance = 0.8/1 = 0.8; rng 0.5 → drop
        let plan = evaluate_alternative_outcome(
            550,
            false,
            true,
            &[],
            -3.0,
            2.0,
            67,
            2.0, // chance 0.4
            10.0,
            5.0,
            0.0, // always fail with negative hits
            0.3, // < 0.4 → drop
        );
        match plan {
            AltOutcomePlan::TryAgain {
                place_id,
                count_obj_after,
                say_fortification,
                hits_after,
                ..
            } => {
                assert_eq!(place_id, Some(67));
                assert_eq!(count_obj_after, Some(1.0));
                assert!(say_fortification);
                assert!((hits_after - (-2.0)).abs() < 1e-5);
            }
            other => panic!("expected TryAgain fort, got {other:?}"),
        }
    }

    #[test]
    fn content_tree_outcomes_resolve() {
        let mut db = ContentDb::default();
        apply_default_alternative_outcome_patches(&mut db);
        let outs = resolve_alternative_outcomes(&db, 71, 340, 340);
        assert!(outs.contains(&344));
        assert!(outs.contains(&345));
        // transition shovel+stump
        let outs = resolve_alternative_outcomes(&db, 502, 338, 0);
        assert_eq!(outs, &[72]);
    }

    #[test]
    fn pick_index_bounds() {
        assert_eq!(pick_outcome_index(0, 0.5), None);
        assert_eq!(pick_outcome_index(3, 0.0), Some(0));
        assert_eq!(pick_outcome_index(3, 0.99), Some(2));
    }
}

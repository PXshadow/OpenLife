//! TH-MULTI-POLISH wire helpers for bare-hand loved-plant extra harvest.
//!
//! Haxe: `TransitionHelper.DoChangeNumberOfUsesOnTarget` loved-food block
//! (actorID==0, non-reverse, target in `player.getLovedPlants()`).

use crate::player::multi_use::{
    is_loved_plant_target, loved_food_bare_hand_gate, loved_food_effective_chance,
    loved_food_extra_hit, loved_food_extra_target_outcome, LovedFoodExtraTarget,
    LOVED_FOOD_USE_CHANCE,
};
use ol_content::ContentDb;
use ol_world::{ComplexObject, World};
use std::sync::Mutex;

/// Result of applying the loved-food extra gate (before numberOfUses change).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LovedFoodWire {
    /// True when player got an extra (plant not consumed this hit).
    pub got_extra: bool,
    /// Hits after this attempt (incremented on extra).
    pub hits: f32,
    /// Target id to place (may restore original plant).
    pub place_target: i32,
    /// When true, skip normal use decrement (`no_use_target`).
    pub force_no_use: bool,
}

/// Conn that just got a loved-food extra (for PS/PE feedback). Taken once by USE handler.
static LAST_LOVED_FOOD_EXTRA: Mutex<Option<u64>> = Mutex::new(None);

/// Record that `conn_id` got Haxe "got an extra!" this USE.
pub fn note_loved_food_extra(conn_id: u64) {
    if let Ok(mut g) = LAST_LOVED_FOOD_EXTRA.lock() {
        *g = Some(conn_id);
    }
}

/// Take and clear the pending loved-food extra conn (if any).
pub fn take_loved_food_extra() -> Option<u64> {
    LAST_LOVED_FOOD_EXTRA.lock().ok().and_then(|mut g| g.take())
}

/// Evaluate loved-food bare-hand extra for a USE that already resolved a transition.
///
/// `actor_id` / `reverse_use_target` / `original_target` / `target_after` match Haxe
/// after `transformHeldObject` + `target.id = newTargetID`.
pub fn evaluate_loved_food_extra(
    content: &ContentDb,
    person_display_id: i32,
    actor_id: i32,
    reverse_use_target: bool,
    original_target: i32,
    target_after: i32,
    hits_before: f32,
    rng01: f32,
) -> LovedFoodWire {
    let mut out = LovedFoodWire {
        got_extra: false,
        hits: hits_before,
        place_target: target_after,
        force_no_use: false,
    };
    if !loved_food_bare_hand_gate(actor_id, reverse_use_target) {
        return out;
    }
    let person = content.person_color(person_display_id);
    if !is_loved_plant_target(person, original_target) {
        return out;
    }
    let chance = loved_food_effective_chance(LOVED_FOOD_USE_CHANCE, hits_before);
    if !loved_food_extra_hit(chance, rng01) {
        return out;
    }
    out.got_extra = true;
    out.hits = hits_before + 1.0;
    let num_after = content
        .get(target_after)
        .map(|d| d.num_uses)
        .unwrap_or(0);
    match loved_food_extra_target_outcome(num_after) {
        LovedFoodExtraTarget::KeepTransformedNoUse => {
            out.force_no_use = true;
            out.place_target = target_after;
        }
        LovedFoodExtraTarget::RestoreOriginal => {
            out.force_no_use = true;
            out.place_target = original_target;
        }
    }
    out
}

/// Apply hits onto the tile helper after place (runtime-only; not OLW2).
pub fn stamp_hits(world: &mut World, tx: i32, ty: i32, hits: f32) {
    if hits <= 0.0 {
        return;
    }
    if let Some(h) = world.helpers.get_mut(&(tx, ty)) {
        h.hits = hits;
        return;
    }
    let base = world.get_object(tx, ty);
    if base != 0 {
        let mut c = ComplexObject::new_simple(base);
        c.hits = hits;
        world.set_object_complex(tx, ty, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::multi_use::PERSON_BROWN;
    use ol_content::ObjectDef;

    #[test]
    fn banana_plant_brown_extra_restores_single_use() {
        let mut db = ContentDb::default();
        db.objects.insert(2142, ObjectDef::empty(2142));
        db.objects.insert(
            2143,
            ObjectDef {
                food_value: 5,
                ..ObjectDef::empty(2143)
            },
        );
        db.person_race.insert(19, PERSON_BROWN);
        let w = evaluate_loved_food_extra(&db, 19, 0, false, 2142, 0, 0.0, 0.9);
        assert!(w.got_extra);
        assert!((w.hits - 1.0).abs() < 1e-5);
        assert_eq!(w.place_target, 2142);
        assert!(w.force_no_use);
    }

    #[test]
    fn multi_use_plant_extra_keeps_transformed_no_use() {
        let mut db = ContentDb::default();
        let mut plant = ObjectDef::empty(2142);
        plant.num_uses = 5;
        db.objects.insert(2142, plant);
        db.person_race.insert(19, PERSON_BROWN);
        let w = evaluate_loved_food_extra(&db, 19, 0, false, 2142, 2142, 0.0, 0.9);
        assert!(w.got_extra);
        assert_eq!(w.place_target, 2142);
        assert!(w.force_no_use);
    }

    #[test]
    fn wrong_race_no_extra() {
        let mut db = ContentDb::default();
        db.objects.insert(2142, ObjectDef::empty(2142));
        db.person_race.insert(19, 4); // White
        let w = evaluate_loved_food_extra(&db, 19, 0, false, 2142, 0, 0.0, 0.9);
        assert!(!w.got_extra);
    }

    #[test]
    fn held_tool_skips_gate() {
        let mut db = ContentDb::default();
        db.objects.insert(2142, ObjectDef::empty(2142));
        db.person_race.insert(19, PERSON_BROWN);
        let w = evaluate_loved_food_extra(&db, 19, 502, false, 2142, 0, 0.0, 0.9);
        assert!(!w.got_extra);
    }

    #[test]
    fn hits_ramp_blocks_extra_at_chance_one() {
        // Haxe: useChance += hits/10; hits=5 → effective 1.0 → rng 0.9 never extra
        let mut db = ContentDb::default();
        db.objects.insert(2142, ObjectDef::empty(2142));
        db.person_race.insert(19, PERSON_BROWN);
        let w = evaluate_loved_food_extra(&db, 19, 0, false, 2142, 0, 5.0, 0.9);
        assert!(!w.got_extra);
        assert!((w.hits - 5.0).abs() < 1e-5);
    }

    #[test]
    fn note_and_take_feedback_flag() {
        // Drain any prior test leakage.
        let _ = take_loved_food_extra();
        assert!(take_loved_food_extra().is_none());
        note_loved_food_extra(42);
        assert_eq!(take_loved_food_extra(), Some(42));
        assert!(take_loved_food_extra().is_none());
    }

    #[test]
    fn normal_roll_no_extra_keeps_target() {
        // rng <= chance → plant transforms/consumes normally (got_extra false)
        let mut db = ContentDb::default();
        db.objects.insert(2142, ObjectDef::empty(2142));
        db.person_race.insert(19, PERSON_BROWN);
        let w = evaluate_loved_food_extra(&db, 19, 0, false, 2142, 0, 0.0, 0.1);
        assert!(!w.got_extra);
        assert_eq!(w.place_target, 0);
        assert!(!w.force_no_use);
    }
}

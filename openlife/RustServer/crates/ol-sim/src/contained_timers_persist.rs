//! Contained-item timer re-arm after OLW load (**CONTAINED-TIMERS-PERSIST** / `rearm_after_load`).
//!
//! Haxe: each `ObjectHelper` under `containedObjects` carries `creationTimeInTicks` +
//! `timeToChange` on disk. Rust keeps a runtime parallel map
//! [`crate::WorldMapTimeState::contained_timers`]; NestedHelper slot times (OLW3)
//! are the on-disk source of truth.
//!
//! After load:
//! 1. Prefer slot `(creation_time, time_to_change)` when `time_to_change > 0`.
//! 2. Clamp `creation_time` to `sim_time` when ahead of the clock
//!    (Haxe `ObjectHelper.ReadFromFile` L268).
//! 3. Else seed `(sim_time, 0.0)` so [`crate::do_time_for_contained`] arms on first scan.
//!
//! On timer updates: write back into `ComplexObject.slots` so the next OLW3 save
//! restores the same progress (no separate HashMap on disk). Slot `uses_remaining`
//! is preferred over wire-id inference when set (multi-use last-use path).
//!
//! Nested-in-nested timers: first-level re-arm is still the runtime map; depth≥2
//! lives on NestedHelper (OLW3) and is ticked by [`crate::tick_nested_helpers_deep`]
//! (**NESTED-IN-NESTED-TIMERS** / `deep_contained`).
//!
//! Anchors: `TimeHelper.doTimeForObject`, `ObjectHelper.WriteToFile` / `ReadFromFile`.

use ol_world::{ComplexObject, NestedHelper, World};
use std::collections::HashMap;

/// Haxe `ObjectHelper.ReadFromFile`: clamp creation when it is ahead of the sim clock.
///
/// `if (newObject.creationTimeInTicks > TimeHelper.tick) creation = tick;`
#[inline]
pub fn clamp_creation_to_sim_time(creation: f32, sim_time: f32) -> f32 {
    if creation > sim_time {
        sim_time
    } else {
        creation
    }
}

/// One slot's runtime timer from NestedHelper meta or a fresh re-arm seed.
///
/// Haxe: loaded `ObjectHelper.timeToChange` / `creationTimeInTicks` on contained.
#[inline]
pub fn timer_from_nested_slot(slot: Option<&NestedHelper>, sim_time: f32) -> (f32, f32) {
    if let Some(s) = slot {
        if s.time_to_change > 0.0 {
            return (
                clamp_creation_to_sim_time(s.creation_time, sim_time),
                s.time_to_change,
            );
        }
        // Creation known but ttc not yet armed (Haxe arms on first doTimeForObject).
        if s.creation_time > 0.0 {
            return (clamp_creation_to_sim_time(s.creation_time, sim_time), 0.0);
        }
    }
    (sim_time, 0.0)
}

/// Explicit NestedHelper uses when set; `0` → caller infers from wire id / dummies.
///
/// Haxe `ObjectHelper.numberOfUses` is persisted on contained ObjectHelpers; OLW3
/// stores it as `NestedHelper.uses_remaining`. Prefer this for last-use selection
/// when multi-use is not (fully) encoded in the wire dummy id.
#[inline]
pub fn uses_from_nested_slot(slot: Option<&NestedHelper>) -> i32 {
    slot.map(|s| s.uses_remaining).unwrap_or(0).max(0)
}

/// Parallel `(creation, ttc)` vector for one container helper (first level only).
pub fn timers_from_helper_for_rearm(h: &ComplexObject, sim_time: f32) -> Vec<(f32, f32)> {
    h.contained
        .iter()
        .enumerate()
        .map(|(i, _)| timer_from_nested_slot(h.slots.get(i), sim_time))
        .collect()
}

/// Parallel uses vector for one container (first level); `0` = infer from wire later.
pub fn uses_from_helper_for_rearm(h: &ComplexObject) -> Vec<i32> {
    h.contained
        .iter()
        .enumerate()
        .map(|(i, _)| uses_from_nested_slot(h.slots.get(i)))
        .collect()
}

/// Scan all world helpers with contained items → runtime timer map.
///
/// Pure; does not mutate the world. Call after OLW load (chunk `rearm_after_load`).
/// First-level slots only for the runtime map; deep NestedHelper times stay on slots.
pub fn rebuild_contained_timers_from_world(
    world: &World,
    sim_time: f32,
) -> HashMap<(i32, i32), Vec<(f32, f32)>> {
    let mut out = HashMap::new();
    for (&(x, y), h) in world.helpers.iter() {
        if h.contained.is_empty() {
            continue;
        }
        out.insert((x, y), timers_from_helper_for_rearm(h, sim_time));
    }
    out
}

/// Write runtime contained timers into NestedHelper slots for OLW3 save.
///
/// Ensures `slots` is parallel to `contained` (synthesizes from wire when empty).
/// Does not clear existing `uses_remaining` on slots.
pub fn apply_contained_timers_to_slots(helper: &mut ComplexObject, timers: &[(f32, f32)]) {
    ensure_slots_parallel(helper);
    for (i, s) in helper.slots.iter_mut().enumerate() {
        s.id = helper.contained[i];
        if let Some(&(cr, tt)) = timers.get(i) {
            s.creation_time = cr;
            s.time_to_change = tt;
        }
    }
}

/// Stamp `uses_remaining` into NestedHelper slots after multi-use transform write-back.
///
/// Length may be shorter than `contained` (only first `uses.len()` slots updated).
/// Negative values are ignored (leave prior uses).
pub fn apply_contained_uses_to_slots(helper: &mut ComplexObject, uses: &[i32]) {
    if uses.is_empty() || helper.contained.is_empty() {
        return;
    }
    ensure_slots_parallel(helper);
    for (i, s) in helper.slots.iter_mut().enumerate() {
        if let Some(&u) = uses.get(i) {
            if u >= 0 {
                s.uses_remaining = u;
            }
        }
    }
}

/// Ensure `slots` is parallel to `contained` (synthesize / pad / truncate).
fn ensure_slots_parallel(helper: &mut ComplexObject) {
    if helper.contained.is_empty() {
        helper.slots.clear();
        return;
    }
    if helper.slots.is_empty() {
        helper.synthesize_slots_from_wire();
    }
    while helper.slots.len() < helper.contained.len() {
        let id = helper.contained[helper.slots.len()];
        helper.slots.push(NestedHelper::id_only(id));
    }
    helper.slots.truncate(helper.contained.len());
}

/// Stats from [`rebuild_contained_timers_from_world`] applied into a map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainedTimerRearmStats {
    pub tiles: usize,
    pub slots: usize,
    pub with_persisted_ttc: usize,
}

/// Count how many re-armed slots had a persisted `time_to_change > 0`.
pub fn rearm_stats(
    world: &World,
    timers: &HashMap<(i32, i32), Vec<(f32, f32)>>,
) -> ContainedTimerRearmStats {
    let mut stats = ContainedTimerRearmStats {
        tiles: timers.len(),
        ..Default::default()
    };
    for ((x, y), ts) in timers {
        stats.slots += ts.len();
        if let Some(h) = world.helpers.get(&(*x, *y)) {
            for (i, &(_cr, tt)) in ts.iter().enumerate() {
                if tt > 0.0 {
                    // Prefer counting from slot meta when present.
                    if h.slots.get(i).map(|s| s.time_to_change > 0.0).unwrap_or(true) {
                        stats.with_persisted_ttc += 1;
                    }
                }
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_world::{ComplexObject, NestedHelper, World};

    #[test]
    fn timer_from_slot_prefers_persisted_ttc() {
        let mut s = NestedHelper::id_only(50);
        s.creation_time = 12.0;
        s.time_to_change = 60.0;
        assert_eq!(timer_from_nested_slot(Some(&s), 100.0), (12.0, 60.0));
    }

    #[test]
    fn timer_from_slot_creation_only_seeds_ttc_zero() {
        let mut s = NestedHelper::id_only(50);
        s.creation_time = 5.0;
        assert_eq!(timer_from_nested_slot(Some(&s), 100.0), (5.0, 0.0));
    }

    #[test]
    fn timer_from_missing_slot_uses_sim_time() {
        assert_eq!(timer_from_nested_slot(None, 42.5), (42.5, 0.0));
    }

    #[test]
    fn timer_from_slot_clamps_creation_ahead_of_sim_time() {
        // Haxe ObjectHelper.ReadFromFile L268.
        let mut s = NestedHelper::id_only(50);
        s.creation_time = 999.0;
        s.time_to_change = 40.0;
        assert_eq!(timer_from_nested_slot(Some(&s), 50.0), (50.0, 40.0));
        s.time_to_change = 0.0;
        s.creation_time = 200.0;
        assert_eq!(timer_from_nested_slot(Some(&s), 10.0), (10.0, 0.0));
        assert_eq!(clamp_creation_to_sim_time(5.0, 10.0), 5.0);
        assert_eq!(clamp_creation_to_sim_time(10.0, 10.0), 10.0);
    }

    #[test]
    fn uses_from_slot_prefers_persisted_uses() {
        let mut s = NestedHelper::id_only(100);
        s.uses_remaining = 1;
        assert_eq!(uses_from_nested_slot(Some(&s)), 1);
        assert_eq!(uses_from_nested_slot(None), 0);
        s.uses_remaining = 0;
        assert_eq!(uses_from_nested_slot(Some(&s)), 0);
    }

    #[test]
    fn rebuild_from_world_reads_olw3_slot_times() {
        let mut world = World::new(8, 8, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50, 51];
        let mut s0 = NestedHelper::id_only(50);
        s0.creation_time = 10.0;
        s0.time_to_change = 30.0;
        let mut s1 = NestedHelper::id_only(51);
        s1.creation_time = 11.0;
        // ttc 0 → re-arm seed keeps creation, ttc=0
        h.slots = vec![s0, s1];
        world.set_object_complex(2, 3, h);

        let map = rebuild_contained_timers_from_world(&world, 99.0);
        let ts = map.get(&(2, 3)).expect("tile timers");
        assert_eq!(ts, &[(10.0, 30.0), (11.0, 0.0)]);
    }

    #[test]
    fn rebuild_id_only_container_seeds_fresh() {
        let mut world = World::new(4, 4, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        // no slots — wire-only (OLW1/2 path)
        world.set_object_complex(0, 0, h);

        let map = rebuild_contained_timers_from_world(&world, 7.0);
        assert_eq!(map.get(&(0, 0)).unwrap(), &[(7.0, 0.0)]);
    }

    #[test]
    fn apply_timers_to_slots_synthesizes_and_stamps() {
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50, 60];
        apply_contained_timers_to_slots(&mut h, &[(1.0, 2.0), (3.0, 4.0)]);
        assert_eq!(h.slots.len(), 2);
        assert_eq!(h.slots[0].id, 50);
        assert!((h.slots[0].creation_time - 1.0).abs() < 1e-5);
        assert!((h.slots[0].time_to_change - 2.0).abs() < 1e-5);
        assert_eq!(h.slots[1].id, 60);
        assert!((h.slots[1].time_to_change - 4.0).abs() < 1e-5);
    }

    #[test]
    fn apply_uses_to_slots_stamps_without_clearing_times() {
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        apply_contained_timers_to_slots(&mut h, &[(10.0, 20.0)]);
        apply_contained_uses_to_slots(&mut h, &[2]);
        assert_eq!(h.slots[0].uses_remaining, 2);
        assert!((h.slots[0].creation_time - 10.0).abs() < 1e-5);
        assert!((h.slots[0].time_to_change - 20.0).abs() < 1e-5);
    }

    #[test]
    fn rearm_roundtrip_slots_survive_rebuild() {
        // Simulate map-slice writing timers into slots, then "load" rebuild.
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![77];
        apply_contained_timers_to_slots(&mut h, &[(100.0, 45.0)]);

        let mut world = World::new(4, 4, false);
        world.set_object_complex(1, 1, h);

        let map = rebuild_contained_timers_from_world(&world, 999.0);
        assert_eq!(map.get(&(1, 1)).unwrap(), &[(100.0, 45.0)]);
        let stats = rearm_stats(&world, &map);
        assert_eq!(stats.tiles, 1);
        assert_eq!(stats.slots, 1);
        assert_eq!(stats.with_persisted_ttc, 1);
    }

    #[test]
    fn nested_in_nested_times_stay_on_slots_not_runtime_map() {
        // Runtime map is first-level only; deep times stay on NestedHelper (deep tick).
        let mut world = World::new(4, 4, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = NestedHelper::id_only(50);
        outer.creation_time = 10.0;
        outer.time_to_change = 30.0;
        let mut inner = NestedHelper::id_only(60);
        inner.creation_time = 11.0;
        inner.time_to_change = 99.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];
        world.set_object_complex(0, 0, h);

        let map = rebuild_contained_timers_from_world(&world, 100.0);
        // Only first-level timer in runtime map.
        assert_eq!(map.get(&(0, 0)).unwrap(), &[(10.0, 30.0)]);
        assert_eq!(map.len(), 1);
        // Deep times still on slots for map-slice deep tick / OLW3 save.
        let h = world.get_helper(0, 0).unwrap();
        assert!((h.slots[0].contained[0].creation_time - 11.0).abs() < 1e-5);
        assert!((h.slots[0].contained[0].time_to_change - 99.0).abs() < 1e-5);
    }

    #[test]
    fn uses_from_helper_for_rearm_reads_slot_uses() {
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![100, 101];
        let mut s0 = NestedHelper::id_only(100);
        s0.uses_remaining = 1;
        let s1 = NestedHelper::id_only(101); // uses 0
        h.slots = vec![s0, s1];
        assert_eq!(uses_from_helper_for_rearm(&h), vec![1, 0]);
    }
}

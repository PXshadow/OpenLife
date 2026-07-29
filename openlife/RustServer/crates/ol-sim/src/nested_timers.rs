//! Nested-in-nested contained timers (**NESTED-IN-NESTED-TIMERS** / `deep_contained`).
//!
//! Haxe `TimeHelper.DoWorldMapTimeStuff` L1149–1168 only walks first-level
//! `containedObjects` and leaves a TODO at L1150:
//! `// TODO time in contained objects in contained objects`.
//!
//! This module implements that TODO for Rust:
//! - First-level still uses the runtime `contained_timers` map + OLW3 slot meta
//! - Depth ≥ 2 lives on [`ol_world::NestedHelper`] (OLW3 recursive slots)
//! - [`tick_nested_helpers_deep`] walks NestedHelper trees with
//!   [`do_time_for_contained`] (Haxe `doTimeForObject`)
//! - Overflow refuse when cargo would exceed new `num_slots` (Haxe L2213)
//!
//! Anchors: `TimeHelper.doTimeForObject`, `ObjectHelper.containedObjects` recursive WriteToFile.
//!
//! Note: loaded as `world_time` submodule via `#[path]` so map-slice can call without
//! a separate top-level `mod` (build also wires top-level re-exports).

use super::{do_time_for_contained, uses_remaining_from_wire_id, ContainedTimeOutcome};
use ol_content::ContentDb;
use ol_world::{ComplexObject, NestedHelper};
use rand::Rng;

/// Max recursion depth for nested-in-nested timer walks (safety).
pub const NESTED_TIMER_MAX_DEPTH: u32 = 8;

/// Apply [`do_time_for_contained`] to NestedHelper trees under first-level slots
/// (and recursively under those).
///
/// - Times live on NestedHelper (OLW3); no separate runtime map for depth≥2.
/// - Prunes `id < 1` after each level (Haxe rebuild of containedObjects).
/// - On transform: keep recursive cargo when `new.num_slots >= cargo.len()`;
///   else refuse transform (Haxe L2213 port-as-is).
/// - Returns `true` when any id/structure changed (needs MX); arm/pending alone is `false`.
pub fn tick_nested_helpers_deep(
    content: &ContentDb,
    nodes: &mut Vec<NestedHelper>,
    sim_time: f32,
    rng: &mut impl Rng,
) -> bool {
    tick_nested_helpers_deep_at(content, nodes, sim_time, rng, 0)
}

fn tick_nested_helpers_deep_at(
    content: &ContentDb,
    nodes: &mut Vec<NestedHelper>,
    sim_time: f32,
    rng: &mut impl Rng,
    depth: u32,
) -> bool {
    if depth > NESTED_TIMER_MAX_DEPTH || nodes.is_empty() {
        return false;
    }
    let mut changed = false;
    let mut keep: Vec<NestedHelper> = Vec::with_capacity(nodes.len());
    for mut node in std::mem::take(nodes) {
        if node.id < 1 {
            changed = true;
            continue;
        }
        let uses = if node.uses_remaining > 0 {
            node.uses_remaining
        } else {
            uses_remaining_from_wire_id(content, node.id)
        };
        let outcome = do_time_for_contained(
            content,
            node.id,
            node.creation_time,
            node.time_to_change,
            sim_time,
            rng.gen(),
            rng.gen(),
            uses,
        );
        match outcome {
            ContainedTimeOutcome::NoTransition => {}
            ContainedTimeOutcome::Pending { creation, ttc } => {
                node.creation_time = creation;
                node.time_to_change = ttc;
            }
            ContainedTimeOutcome::Transformed {
                new_id,
                creation,
                ttc,
                uses_remaining: out_uses,
            } => {
                if new_id <= 0 {
                    changed = true;
                    continue;
                }
                // Haxe L2213: refuse transform when cargo would overflow new slots.
                let new_base = content.resolve_base_id(new_id);
                let num_slots = content
                    .get(new_base)
                    .map(|d| d.num_slots.max(0) as usize)
                    .unwrap_or(0);
                if !node.contained.is_empty() && node.contained.len() > num_slots {
                    keep.push(node);
                    continue;
                }
                changed = true;
                node.id = new_id;
                node.creation_time = creation;
                node.time_to_change = ttc;
                node.uses_remaining = out_uses;
            }
        }
        if tick_nested_helpers_deep_at(content, &mut node.contained, sim_time, rng, depth + 1) {
            changed = true;
        }
        keep.push(node);
    }
    *nodes = keep;
    changed
}

/// Ensure first-level NestedHelper slots carry wire nest as deep children when needed.
fn ensure_slot_deep_from_wire(helper: &mut ComplexObject) {
    if helper.slots.is_empty() {
        if !helper.contained.is_empty() {
            helper.synthesize_slots_from_wire();
        }
        return;
    }
    if helper.nested.is_empty() {
        return;
    }
    for (i, s) in helper.slots.iter_mut().enumerate() {
        if !s.contained.is_empty() {
            continue;
        }
        if let Some(nest) = helper.nested.get(i) {
            if !nest.is_empty() {
                s.contained = nest.iter().copied().map(NestedHelper::id_only).collect();
            }
        }
    }
}

/// Process first-level contained timers + nested-in-nested deep timers on one helper.
///
/// Updates `helper` (contained / nested / slots) and `timers` (first-level runtime map).
/// Returns `true` when MX is needed (id/structure change).
///
/// Haxe: first-level `doTimeForObject` loop + L1150 nested-in-nested (implemented).
pub fn tick_container_helper_timers(
    content: &ContentDb,
    helper: &mut ComplexObject,
    timers: &mut Vec<(f32, f32)>,
    sim_time: f32,
    rng: &mut impl Rng,
) -> bool {
    if helper.contained.is_empty() {
        timers.clear();
        return false;
    }
    let n = helper.contained.len();
    while timers.len() < n {
        timers.push((sim_time, 0.0));
    }
    timers.truncate(n);

    ensure_slot_deep_from_wire(helper);

    let mut changed = false;
    let mut new_contained = Vec::with_capacity(n);
    let mut new_timers = Vec::with_capacity(n);
    let mut new_slots: Vec<NestedHelper> = Vec::with_capacity(n);

    let old_slots = std::mem::take(&mut helper.slots);
    let old_nested = std::mem::take(&mut helper.nested);
    let old_contained: Vec<i32> = std::mem::take(&mut helper.contained);

    for (i, cid) in old_contained.into_iter().enumerate() {
        let (cr, tt) = timers.get(i).copied().unwrap_or((sim_time, 0.0));
        let mut slot = old_slots
            .get(i)
            .cloned()
            .unwrap_or_else(|| NestedHelper::id_only(cid));
        if slot.contained.is_empty() {
            if let Some(nest) = old_nested.get(i) {
                if !nest.is_empty() {
                    slot.contained = nest.iter().copied().map(NestedHelper::id_only).collect();
                }
            }
        }
        slot.id = cid;

        let slot_uses = crate::contained_timers_persist::uses_from_nested_slot(Some(&slot));
        let uses = if slot_uses > 0 {
            slot_uses
        } else {
            uses_remaining_from_wire_id(content, cid)
        };
        let prior_uses = if slot_uses > 0 { slot_uses } else { uses };

        let outcome = do_time_for_contained(
            content,
            cid,
            cr,
            tt,
            sim_time,
            rng.gen(),
            rng.gen(),
            uses,
        );

        match outcome {
            ContainedTimeOutcome::NoTransition => {
                if tick_nested_helpers_deep(content, &mut slot.contained, sim_time, rng) {
                    changed = true;
                }
                slot.id = cid;
                slot.creation_time = cr;
                slot.time_to_change = tt;
                if prior_uses > 0 {
                    slot.uses_remaining = prior_uses;
                }
                new_contained.push(cid);
                new_timers.push((cr, tt));
                new_slots.push(slot);
            }
            ContainedTimeOutcome::Pending { creation, ttc } => {
                if tick_nested_helpers_deep(content, &mut slot.contained, sim_time, rng) {
                    changed = true;
                }
                slot.id = cid;
                slot.creation_time = creation;
                slot.time_to_change = ttc;
                if prior_uses > 0 {
                    slot.uses_remaining = prior_uses;
                }
                new_contained.push(cid);
                new_timers.push((creation, ttc));
                new_slots.push(slot);
            }
            ContainedTimeOutcome::Transformed {
                new_id,
                creation,
                ttc,
                uses_remaining: out_uses,
            } => {
                if new_id > 0 {
                    let new_base = content.resolve_base_id(new_id);
                    let num_slots = content
                        .get(new_base)
                        .map(|d| d.num_slots.max(0) as usize)
                        .unwrap_or(0);
                    if !slot.contained.is_empty() && slot.contained.len() > num_slots {
                        // Refuse first-level transform (Haxe L2213); still deep-tick cargo.
                        if tick_nested_helpers_deep(content, &mut slot.contained, sim_time, rng) {
                            changed = true;
                        }
                        slot.id = cid;
                        slot.creation_time = cr;
                        slot.time_to_change = tt;
                        if prior_uses > 0 {
                            slot.uses_remaining = prior_uses;
                        }
                        new_contained.push(cid);
                        new_timers.push((cr, tt));
                        new_slots.push(slot);
                    } else {
                        changed = true;
                        slot.id = new_id;
                        slot.creation_time = creation;
                        slot.time_to_change = ttc;
                        slot.uses_remaining = out_uses;
                        let _ = tick_nested_helpers_deep(
                            content,
                            &mut slot.contained,
                            sim_time,
                            rng,
                        );
                        new_contained.push(new_id);
                        new_timers.push((creation, ttc));
                        new_slots.push(slot);
                    }
                } else {
                    changed = true;
                }
            }
        }
    }

    helper.contained = new_contained;
    helper.slots = new_slots;
    helper.rebuild_wire_from_slots();
    *timers = new_timers;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef, Transition};
    use ol_world::World;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn tr_decay(target: i32, new_target: i32, secs: f32) -> Transition {
        Transition {
            actor_id: -1,
            target_id: target,
            new_actor_id: 0,
            new_target_id: new_target,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: secs,
            reverse_use_actor: false,
            reverse_use_target: false,
            no_use_actor: false,
            no_use_target: false,
            move_dist: 0,
            desired_move_dist: 0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 0.0,
            switch_number_of_uses: false,
            target_number_of_uses: -1,
            is_pickup_or_drop: false,
        }
    }

    #[test]
    fn tick_nested_helpers_deep_transforms_inner() {
        let mut db = ContentDb::default();
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 1.0));

        let mut inner = NestedHelper::id_only(60);
        inner.creation_time = 0.0;
        inner.time_to_change = 1.0;
        let mut nodes = vec![inner];
        let mut rng = StdRng::seed_from_u64(9);
        let changed = tick_nested_helpers_deep(&db, &mut nodes, 10.0, &mut rng);
        assert!(changed);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, 61);
    }

    #[test]
    fn tick_nested_helpers_deep_prunes_empty_and_recurses() {
        let mut db = ContentDb::default();
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 2,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(70, ObjectDef::empty(70));
        db.objects.insert(71, ObjectDef::empty(71));
        db.auto_decays.insert(70, tr_decay(70, 0, 1.0));

        let mut mid = NestedHelper::id_only(70);
        mid.creation_time = 0.0;
        mid.time_to_change = 1.0;
        let mut deep = NestedHelper::id_only(71);
        deep.creation_time = 0.0;
        deep.time_to_change = 99.0;
        mid.contained = vec![deep];
        let mut nodes = vec![mid, NestedHelper::id_only(0)];
        let mut rng = StdRng::seed_from_u64(10);
        let changed = tick_nested_helpers_deep(&db, &mut nodes, 10.0, &mut rng);
        assert!(changed);
        assert!(nodes.is_empty(), "decayed + empty pruned");
    }

    #[test]
    fn nested_in_nested_overflow_refuses_parent_transform() {
        let mut db = ContentDb::default();
        db.objects.insert(
            60,
            ObjectDef {
                id: 60,
                num_slots: 2,
                ..ObjectDef::empty(60)
            },
        );
        db.objects.insert(
            61,
            ObjectDef {
                id: 61,
                num_slots: 0,
                ..ObjectDef::empty(61)
            },
        );
        db.objects.insert(70, ObjectDef::empty(70));
        db.auto_decays.insert(60, tr_decay(60, 61, 1.0));

        let cargo = NestedHelper::id_only(70);
        let mut parent = NestedHelper::id_only(60);
        parent.creation_time = 0.0;
        parent.time_to_change = 1.0;
        parent.contained = vec![cargo];
        let mut nodes = vec![parent];
        let mut rng = StdRng::seed_from_u64(13);
        let _changed = tick_nested_helpers_deep(&db, &mut nodes, 10.0, &mut rng);
        assert_eq!(nodes[0].id, 60, "overflow must refuse transform");
        assert_eq!(nodes[0].contained.len(), 1);
    }

    #[test]
    fn tick_container_helper_deep_transform() {
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 1.0));

        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = NestedHelper::id_only(50);
        let mut inner = NestedHelper::id_only(60);
        inner.creation_time = 0.0;
        inner.time_to_change = 1.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];
        h.rebuild_wire_from_slots();

        let mut timers = vec![(0.0_f32, 0.0_f32)];
        let mut rng = StdRng::seed_from_u64(11);
        let changed = tick_container_helper_timers(&db, &mut h, &mut timers, 10.0, &mut rng);
        assert!(changed);
        assert_eq!(h.contained, vec![50]);
        assert_eq!(h.nested, vec![vec![61]]);
        assert_eq!(h.slots[0].contained[0].id, 61);
    }

    #[test]
    fn tick_container_helper_first_level_still_transforms() {
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(50, ObjectDef::empty(50));
        db.objects.insert(51, ObjectDef::empty(51));
        db.auto_decays.insert(50, tr_decay(50, 51, 1.0));

        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut timers = vec![(0.0_f32, 1.0_f32)];
        let mut rng = StdRng::seed_from_u64(2);
        let changed = tick_container_helper_timers(&db, &mut h, &mut timers, 10.0, &mut rng);
        assert!(changed);
        assert_eq!(h.contained, vec![51]);
    }

    #[test]
    fn nested_in_nested_mid_ttc_survives() {
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 60.0));

        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = NestedHelper::id_only(50);
        let mut inner = NestedHelper::id_only(60);
        inner.creation_time = 100.0;
        inner.time_to_change = 60.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];

        let mut timers = vec![(100.0_f32, 0.0_f32)];
        let mut rng = StdRng::seed_from_u64(12);
        let changed = tick_container_helper_timers(&db, &mut h, &mut timers, 140.0, &mut rng);
        assert!(!changed);
        assert_eq!(h.slots[0].contained[0].id, 60);
        assert!((h.slots[0].contained[0].creation_time - 100.0).abs() < 1e-5);
        assert!((h.slots[0].contained[0].time_to_change - 60.0).abs() < 1e-5);

        let changed2 = tick_container_helper_timers(&db, &mut h, &mut timers, 165.0, &mut rng);
        assert!(changed2);
        assert_eq!(h.slots[0].contained[0].id, 61);
    }

    #[test]
    fn deep_times_stay_on_slots_not_runtime_map() {
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

        let map =
            crate::contained_timers_persist::rebuild_contained_timers_from_world(&world, 100.0);
        assert_eq!(map.get(&(0, 0)).unwrap(), &[(10.0, 30.0)]);
        let h = world.get_helper(0, 0).unwrap();
        assert!((h.slots[0].contained[0].creation_time - 11.0).abs() < 1e-5);
        assert!((h.slots[0].contained[0].time_to_change - 99.0).abs() < 1e-5);
    }
}

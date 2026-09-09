/// Haxe `ServerSettings.PatchTransitions` — `TransitionData.hungryWorkCost`.
///
/// Not in transition files / OLT1 (default 0). `hungryWorkTemperature` stays −1
/// (Haxe: if below zero, cost × HungryWorkHeat).
///
/// Safe for partial unit-test DBs: `getTransition` rows skip when missing;
/// `new TransitionData` rows insert if absent.
// Haxe: ServerSettings.PatchTransitions hungryWorkCost ~2263–3849
// Haxe: TransitionData.hungryWorkCost / hungryWorkTemperature

/// Live `ServerSettings.HungryWorkCost` baked into two shovel+clay-pit rows.
// Haxe: ServerSettings.HungryWorkCost default 5
pub const DEFAULT_HUNGRY_WORK_COST_KNOB: f32 = 5.0;

/// Options for [`apply_default_hungry_work_cost_patches_ex`].
#[derive(Debug, Clone, Copy)]
pub struct HungryWorkCostPatchOpts {
    /// Haxe `ServerSettings.HungryWorkCost` for shovel 502 + Empty Clay Pit 408.
    pub hungry_work_cost_knob: f32,
}

impl Default for HungryWorkCostPatchOpts {
    fn default() -> Self {
        Self {
            hungry_work_cost_knob: DEFAULT_HUNGRY_WORK_COST_KNOB,
        }
    }
}

/// Apply default PatchTransitions hungryWorkCost table (HungryWorkCost = 5).
// Haxe: ServerSettings.PatchTransitions hungryWorkCost
pub fn apply_default_hungry_work_cost_patches(db: &mut ContentDb) {
    apply_default_hungry_work_cost_patches_ex(db, HungryWorkCostPatchOpts::default());
}

/// Apply hungryWorkCost patches with the live HungryWorkCost knob.
// Haxe: ServerSettings.PatchTransitions + HungryWorkCost
pub fn apply_default_hungry_work_cost_patches_ex(
    db: &mut ContentDb,
    opts: HungryWorkCostPatchOpts,
) {
    let knob = if opts.hungry_work_cost_knob.is_finite() {
        opts.hungry_work_cost_knob
    } else {
        DEFAULT_HUNGRY_WORK_COST_KNOB
    };

    // new TransitionData (insert if missing, set cost if present).
    // Haxe: PatchTransitions ~2259–2272, ~2403, ~2567–2574
    upsert_hungry_new(db, 684, 650, 684, 4102, 10.0, false); // pick + bear cave
    upsert_hungry_new(db, 684, 650, 858, 4102, 10.0, true); // last-use actor broken tool
    upsert_hungry_new(db, 34, 32, 33, 32, 1.0, false); // Sharp Stone + Big Hard Rock
    upsert_hungry_new(db, 248, 2156, 67, 86, 5.0, false); // Firebrand + Mosquito Swarm
    upsert_hungry_new(db, 248, 2157, 0, 86, 3.0, false); // Firebrand + Mosquito just bit

    // getTransition (skip if the pair is absent from this DB).
    // Haxe: PatchTransitions ~2513–2612
    patch_hungry_primary(db, 502, 122, 5.0); // Shovel + Tule Stumps
    patch_hungry_primary(db, 0, 125, 3.0); // empty + Clay Deposit
    patch_hungry_primary(db, 0, 409, 3.0); // empty + Clay Pit
    patch_hungry_primary(db, 502, 32, 10.0); // Shovel + Big Hard Rock
    patch_hungry_primary(db, 291, 486, 5.0); // Flat Rock + Floor Stakes
    patch_hungry_primary(db, 684, 1596, 5.0); // Steel Mining Pick + Stone Road
    patch_hungry_primary(db, 684, 896, 10.0); // pick + Ancient Stone Wall H
    patch_hungry_primary(db, 684, 895, 10.0); // pick + Ancient Stone Wall C
    patch_hungry_primary(db, 684, 897, 10.0); // pick + Ancient Stone Wall V
    patch_hungry_primary(db, 462, 2757, 5.0); // Steel Adze + Springy Wooden Door H
    patch_hungry_primary(db, 462, 2759, 5.0); // Steel Adze + Springy Wooden Door V
    patch_hungry_primary(db, 1137, 848, -5.0); // Bowl of Soil + Hardened Row (no hungry)
    patch_hungry_primary(db, 467, 508, 10.0); // Mallet + Dug Big Rock with Chisel
    // Haxe duplicates the 502+408 assignment; apply once.
    patch_hungry_primary(db, 502, 408, knob); // Shovel + Empty Clay Pit
    patch_hungry_primary(db, 334, 2145, 10.0); // Steel Axe + Empty Banana Plant
    patch_hungry_primary(db, 684, 680, 10.0); // Steel Mining Pick + Gold Vein

    // Property Gate 2962: skip empty-hand; 0.1 then 5 except skewers 139/852.
    // Haxe: PatchTransitions ~3835–3849 GetTransitionByTarget(2962)
    patch_hungry_work_cost_by_target(db, 2962);
}

fn patch_hungry_primary(db: &mut ContentDb, actor: i32, target: i32, cost: f32) {
    if let Some(t) = db.transitions.get_mut(&(actor, target)) {
        t.hungry_work_cost = cost;
    }
}

fn upsert_hungry_new(
    db: &mut ContentDb,
    actor: i32,
    target: i32,
    new_actor: i32,
    new_target: i32,
    cost: f32,
    last_use_actor: bool,
) {
    let key = (actor, target);
    let table = if last_use_actor {
        &mut db.transitions_last_use
    } else {
        &mut db.transitions
    };
    if let Some(t) = table.get_mut(&key) {
        t.hungry_work_cost = cost;
        return;
    }
    table.insert(
        key,
        Transition {
            actor_id: actor,
            target_id: target,
            new_actor_id: new_actor,
            new_target_id: new_target,
            last_use_actor,
            hungry_work_cost: cost,
            ..Default::default()
        },
    );
    if last_use_actor {
        db.last_use_transition_count = db.last_use_transition_count.saturating_add(1);
    } else {
        db.transition_count = db.transition_count.saturating_add(1);
    }
}

/// Haxe `GetTransitionByTarget` hungryWorkCost for Property Gate (and tests).
///
/// Skip empty-hand (`actorID == 0`). Others get 0.1; non-skewer (not 139/852) get 5.
// Haxe: ServerSettings.PatchTransitions ~3835–3849
pub fn patch_hungry_work_cost_by_target(db: &mut ContentDb, target_id: i32) {
    const SKEWER: i32 = 139;
    const WEAK_SKEWER: i32 = 852;
    let patch_table = |table: &mut HashMap<(i32, i32), Transition>| {
        for t in table.values_mut() {
            if t.target_id != target_id {
                continue;
            }
            if t.actor_id == 0 {
                continue;
            }
            t.hungry_work_cost = 0.1;
            if t.actor_id == SKEWER || t.actor_id == WEAK_SKEWER {
                continue;
            }
            t.hungry_work_cost = 5.0;
        }
    };
    patch_table(&mut db.transitions);
    patch_table(&mut db.transitions_last_use);
    patch_table(&mut db.transitions_max_use);
}

#[cfg(test)]
mod hungry_work_cost_patch_tests {
    use super::*;

    fn bare_tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
            ..Default::default()
        }
    }

    #[test]
    fn new_transition_bodies_insert_costs() {
        let mut db = ContentDb::default();
        apply_default_hungry_work_cost_patches(&mut db);
        let bear = db.transitions.get(&(684, 650)).expect("bear cave");
        assert!((bear.hungry_work_cost - 10.0).abs() < 1e-6);
        assert_eq!(bear.new_target_id, 4102);
        let last = db
            .transitions_last_use
            .get(&(684, 650))
            .expect("bear cave last-use");
        assert!(last.last_use_actor);
        assert!((last.hungry_work_cost - 10.0).abs() < 1e-6);
        assert_eq!(last.new_actor_id, 858);
        let mosq = db.transitions.get(&(248, 2156)).expect("mosquito");
        assert!((mosq.hungry_work_cost - 5.0).abs() < 1e-6);
        assert_eq!(mosq.new_actor_id, 67);
        let bit = db.transitions.get(&(248, 2157)).expect("mosquito bit");
        assert!((bit.hungry_work_cost - 3.0).abs() < 1e-6);
        assert!((db.transitions.get(&(34, 32)).unwrap().hungry_work_cost - 1.0).abs() < 1e-6);
        assert!((db.transitions.get(&(248, 2156)).unwrap().hungry_work_temperature + 1.0).abs() < 1e-6);
    }

    #[test]
    fn get_transition_rows_patch_when_present() {
        let mut db = ContentDb::default();
        db.transitions.insert((502, 122), bare_tr(502, 122, 0, 0));
        db.transitions.insert((1137, 848), bare_tr(1137, 848, 0, 0));
        db.transitions.insert((502, 408), bare_tr(502, 408, 0, 0));
        db.transitions.insert((0, 125), bare_tr(0, 125, 0, 0));
        apply_default_hungry_work_cost_patches_ex(
            &mut db,
            HungryWorkCostPatchOpts {
                hungry_work_cost_knob: 9.0,
            },
        );
        assert!((db.transitions.get(&(502, 122)).unwrap().hungry_work_cost - 5.0).abs() < 1e-6);
        assert!((db.transitions.get(&(1137, 848)).unwrap().hungry_work_cost + 5.0).abs() < 1e-6);
        assert!((db.transitions.get(&(502, 408)).unwrap().hungry_work_cost - 9.0).abs() < 1e-6);
        assert!((db.transitions.get(&(0, 125)).unwrap().hungry_work_cost - 3.0).abs() < 1e-6);
        // missing getTransition row stays absent
        assert!(db.transitions.get(&(502, 32)).is_none());
    }

    #[test]
    fn property_gate_skips_empty_hand_and_skewers() {
        let mut db = ContentDb::default();
        db.transitions.insert((0, 2962), bare_tr(0, 2962, 0, 0));
        db.transitions.insert((139, 2962), bare_tr(139, 2962, 0, 0));
        db.transitions.insert((852, 2962), bare_tr(852, 2962, 0, 0));
        db.transitions.insert((34, 2962), bare_tr(34, 2962, 0, 0));
        db.transitions_last_use
            .insert((502, 2962), bare_tr(502, 2962, 0, 0));
        apply_default_hungry_work_cost_patches(&mut db);
        assert!((db.transitions.get(&(0, 2962)).unwrap().hungry_work_cost - 0.0).abs() < 1e-6);
        assert!((db.transitions.get(&(139, 2962)).unwrap().hungry_work_cost - 0.1).abs() < 1e-6);
        assert!((db.transitions.get(&(852, 2962)).unwrap().hungry_work_cost - 0.1).abs() < 1e-6);
        assert!((db.transitions.get(&(34, 2962)).unwrap().hungry_work_cost - 5.0).abs() < 1e-6);
        assert!(
            (db.transitions_last_use.get(&(502, 2962)).unwrap().hungry_work_cost - 5.0).abs()
                < 1e-6
        );
    }
}

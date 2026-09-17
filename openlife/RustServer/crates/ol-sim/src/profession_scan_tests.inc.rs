// Tests for profession_scan (included into short_craft_intent::profession_scan).
use super::*;
use crate::baker_profession::{
    BakeAction, BakerProfessionRuntime, BakerTaskState, BOWL_GOOSEBERRIES, BOWL_TOMATO_SEEDS,
    CLAY_BOWL, CLAY_PLATE, COOKED_TURKEY, FILL_BERRY_HELD_RUNG, GET_CLOSEST_OBJECT_BY_ID_DEFAULT_DIST,
    HOT_OVEN, RAW_MUTTON, SAUERKRAUT, WILD_BUSH,
};
use crate::farmer_profession::{
    basic_farmer_weight_from_runtime, carrot_max_for_dispatch, watering_max_for_dispatch,
    FarmAction, FarmProfession, FarmProfessionRuntime, FarmTaskState, BOWL_OF_GREEN_BEANS,
    BOWL_OF_SOIL, CARROT_ROW, DRY_PLANTED_CARROTS, DRY_PLANTED_WHEAT, DYING_BUSH,
    COOKED_OMELETTE_RUNG, CRITICAL_STUFF_RUNG, PLACE_FLOOR_UNDER_RUNG,
    DO_CARROT_LOW_RUNG, DO_WATERING_LOW_RUNG, FILL_BEAN_BOWL_RUNG, FILL_BEAN_HELD_RUNG,
    GREEN_BEAN_PLANTS, PULL_CARROT_ROW_RUNG,
    WATER_BRINGER_ASSIGNED_MAX_PEOPLE, WATER_BRINGER_LOW_MAX_PEOPLE, WET_PLANTED_CARROTS,
    WET_PLANTED_WHEAT, CARROT_FARMER_LOW_MAX_PEOPLE,
};
use crate::short_craft_intent::ShortCraftLiveIntent;
use crate::smith_profession::{
    SmithAction, SmithProfessionRuntime, BIG_CHARCOAL_PILE, FIRED_BOWL_TONGS, FIRED_NOZZLE_TONGS,
    FIRING_FORGE, FIRING_KILN, FLAT_ROCK, HOT_IRON_BLOOM_FLAT, SMITHING_HAMMER,
    UNFORGED_SEALED_CRUCIBLE, WET_CLAY_BOWL, WET_CLAY_NOZZLE,
};
use crate::{AgeRotatedJobKind, PriorityRung};
use ol_world::World;

fn mock_world_with(objs: &[(i32, i32, i32)]) -> World {
    let mut w = World::new(128, 128, false);
    for &(id, x, y) in objs {
        w.set_object(x, y, id);
    }
    w
}

#[test]
fn scan_held_hungry_work_cost_from_content_pair() {
    use ol_content::{ContentDb, ObjectDef, Transition};
    let mut db = ContentDb::default();
    let mut actor = ObjectDef::empty(100);
    actor.description = "Thing +hungryWork".into();
    db.objects.insert(100, actor);
    db.objects.insert(1845, ObjectDef::empty(1845));
    db.transitions.insert(
        (100, -1),
        Transition {
            actor_id: 100,
            target_id: -1,
            new_actor_id: 0,
            new_target_id: 1845,
            ..Default::default()
        },
    );
    // held 100 +hungryWork knob 7 + fence new_target 5
    assert!((scan_held_hungry_work_cost(&db, 100, 7.0) - 12.0).abs() < 1e-6);
    // empty hands, no trans
    assert!((scan_held_hungry_work_cost(&db, 0, 5.0) - 0.0).abs() < 1e-6);
}

#[test]
fn farm_short_craft_pair_hungry_cost_refuses_when_target_costly() {
    use std::sync::Arc;
    use ol_content::{ContentDb, ObjectDef, Transition};
    let mut db = ContentDb::default();
    db.objects.insert(34, ObjectDef::empty(34));
    db.objects.insert(1, ObjectDef::empty(1));
    db.objects.insert(3146, ObjectDef::empty(3146));
    db.transitions.insert(
        (34, 1),
        Transition {
            actor_id: 34,
            target_id: 1,
            new_actor_id: 34,
            new_target_id: 3146,
            ..Default::default()
        },
    );
    let tiles = vec![ScanTile::simple(1, 3, 4)];
    let mut inp = ProfessionScanInput::basic(0, 0, 34);
    inp.food_store = 3.0; // < knob 5 + 1
    inp.transition_hungry_cost = 0.0; // held (34,-1) free
    inp.hungry_work_cost_knob = 5.0;
    inp.content = Some(Arc::new(db));
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ShortCraft {
            actor: 34,
            target: 1,
        },
        &mut FarmProfessionRuntime::default(),
    );
    assert!(r.had_action);
    assert_eq!(r.intent, ShortCraftLiveIntent::RefuseHungry);

    let mut inp_ok = inp.clone();
    inp_ok.food_store = 20.0;
    let r_ok = farm_action_to_live_intent(
        &tiles,
        &inp_ok,
        FarmAction::ShortCraft {
            actor: 34,
            target: 1,
        },
        &mut FarmProfessionRuntime::default(),
    );
    assert_eq!(
        r_ok.intent,
        ShortCraftLiveIntent::UseAt {
            x: 3,
            y: 4,
            target_id: 1,
            actor_id: 34,
        }
    );
}

#[test]
fn farm_short_craft_without_content_keeps_scan_wide_cost() {
    let tiles = vec![ScanTile::simple(DYING_BUSH, 3, 4)];
    let mut inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    inp.food_store = 1.0;
    inp.transition_hungry_cost = 2.0;
    inp.content = None;
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        },
        &mut FarmProfessionRuntime::default(),
    );
    assert_eq!(r.intent, ShortCraftLiveIntent::RefuseHungry);
}

fn pair_hungry_cost_content_held_free_target_costly() -> ol_content::ContentDb {
    use ol_content::{ContentDb, ObjectDef, Transition};
    let mut db = ContentDb::default();
    db.objects.insert(34, ObjectDef::empty(34));
    db.objects.insert(1, ObjectDef::empty(1));
    db.objects.insert(3146, ObjectDef::empty(3146));
    db.transitions.insert(
        (34, 1),
        Transition {
            actor_id: 34,
            target_id: 1,
            new_actor_id: 34,
            new_target_id: 3146,
            ..Default::default()
        },
    );
    db
}

#[test]
fn baker_short_craft_pair_hungry_cost_refuses_when_target_costly() {
    use std::sync::Arc;
    let tiles = vec![ScanTile::simple(1, 3, 4)];
    let mut inp = ProfessionScanInput::basic(0, 0, 34);
    inp.food_store = 3.0;
    inp.transition_hungry_cost = 0.0;
    inp.hungry_work_cost_knob = 5.0;
    inp.content = Some(Arc::new(pair_hungry_cost_content_held_free_target_costly()));
    let r = bake_action_to_live_intent(
        &tiles,
        &inp,
        BakeAction::ShortCraft {
            actor: 34,
            target: 1,
        },
    );
    assert_eq!(r.intent, ShortCraftLiveIntent::RefuseHungry);

    inp.food_store = 20.0;
    let r_ok = bake_action_to_live_intent(
        &tiles,
        &inp,
        BakeAction::ShortCraft {
            actor: 34,
            target: 1,
        },
    );
    assert!(!matches!(r_ok.intent, ShortCraftLiveIntent::RefuseHungry));
}

#[test]
fn smith_short_craft_pair_hungry_cost_refuses_when_target_costly() {
    use std::sync::Arc;
    let tiles = vec![ScanTile::simple(1, 3, 4)];
    let mut inp = ProfessionScanInput::basic(0, 0, 34);
    inp.food_store = 3.0;
    inp.transition_hungry_cost = 0.0;
    inp.hungry_work_cost_knob = 5.0;
    inp.content = Some(Arc::new(pair_hungry_cost_content_held_free_target_costly()));
    let r = smith_action_to_live_intent(
        &tiles,
        &inp,
        SmithAction::ShortCraft {
            actor: 34,
            target: 1,
        },
    );
    assert_eq!(r.intent, ShortCraftLiveIntent::RefuseHungry);

    inp.food_store = 20.0;
    let r_ok = smith_action_to_live_intent(
        &tiles,
        &inp,
        SmithAction::ShortCraft {
            actor: 34,
            target: 1,
        },
    );
    assert!(!matches!(r_ok.intent, ShortCraftLiveIntent::RefuseHungry));
}

#[test]
fn scan_world_radius_includes_empty_and_objects() {
    let w = mock_world_with(&[(DYING_BUSH, 5, 5), (663, 8, 5)]);
    let tiles = scan_world_radius(&w, None, 5, 5, 3);
    assert!(tiles
        .iter()
        .any(|t| t.parent_id == DYING_BUSH && t.x == 5 && t.y == 5));
    assert!(tiles.iter().any(|t| t.parent_id == 663));
    assert!(tiles.iter().any(|t| t.parent_id == 0));
    assert_eq!(tiles.len(), 7 * 7);
}

#[test]
fn scan_world_radius_r40_is_fast_square() {
    let w = mock_world_with(&[(DYING_BUSH, 0, 0)]);
    let t0 = std::time::Instant::now();
    let tiles = scan_world_radius(&w, None, 0, 0, 40);
    let us = t0.elapsed().as_micros();
    assert_eq!(tiles.len(), 81 * 81);
    assert!(
        us < 50_000,
        "r=40 scan should stay under 50 ms after chunk reuse, was {us} µs"
    );
}

#[test]
fn filter_scan_tiles_in_radius_keeps_inner_square() {
    let w = mock_world_with(&[(DYING_BUSH, 0, 0)]);
    let wide = scan_world_radius(&w, None, 0, 0, 4);
    assert_eq!(wide.len(), 9 * 9);
    let inner = filter_scan_tiles_in_radius(&wide, 0, 0, 1);
    assert_eq!(inner.len(), 3 * 3);
    assert!(inner.iter().any(|t| t.parent_id == DYING_BUSH));
}

#[test]
fn closest_by_parent_and_well_empty() {
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 10, 10),
        ScanTile::simple(DYING_BUSH, 12, 10),
        ScanTile::simple(663, 0, 0),
        ScanTile::empty(1, 0, 0, 0),
        ScanTile::empty(0, 1, 0, 0),
        ScanTile::simple(662, 50, 50),
    ];
    let c = closest_by_parent_id(&tiles, DYING_BUSH, 11, 10, 20).unwrap();
    assert_eq!((c.x, c.y), (10, 10));
    let well = closest_well(&tiles, 0, 0, 60).unwrap();
    assert_eq!(well.parent_id, 663);
    let e = empty_near_well(&tiles, 0, 0, 60);
    assert!(e.is_some());
    // Haxe minDistance: skip near bush, pick farther
    let far = closest_by_parent_id_ex(&tiles, DYING_BUSH, 10, 10, 20, 2).unwrap();
    assert_eq!((far.x, far.y), (12, 10));
    // Target-relative: closer to (13,10) is the (12,10) bush
    let rel =
        closest_by_parent_id_to_target(&tiles, DYING_BUSH, 13, 10, 10, 0).unwrap();
    assert_eq!((rel.x, rel.y), (12, 10));
}

#[test]
fn farm_map_and_soil_on_dying_bush_use_at() {
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 3, 4),
        ScanTile::empty(1, 1, 0, 0),
        ScanTile::empty(2, 2, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let action = FarmAction::ShortCraft {
        actor: BOWL_OF_SOIL,
        target: DYING_BUSH,
    };
    let mut farm_rt = FarmProfessionRuntime::default();
    let r = farm_action_to_live_intent(&tiles, &inp, action, &mut farm_rt);
    assert!(r.had_action);
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::UseAt {
            x: 3,
            y: 4,
            target_id: DYING_BUSH,
            actor_id: BOWL_OF_SOIL,
        }
    );
}

#[test]
fn farm_short_craft_missing_target_returns_false() {
    // Haxe L2722: shortCraftOnTarget(null) → false (no seek of missing target)
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        },
        &mut FarmProfessionRuntime::default(),
    );
    assert!(!r.had_action);
    assert_eq!(r.intent, ShortCraftLiveIntent::None);
}

#[test]
fn farm_drop_held_when_actor_zero() {
    let tiles = vec![
        ScanTile::simple(DRY_PLANTED_CARROTS, 5, 5),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 999);
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ShortCraft {
            actor: 0,
            target: DRY_PLANTED_CARROTS,
        },
        &mut FarmProfessionRuntime::default(),
    );
    assert!(r.had_action);
    assert!(matches!(r.intent, ShortCraftLiveIntent::DropAt { .. }));
}

#[test]
fn smith_hammer_bloom_use_at_from_scan() {
    let tiles = vec![
        ScanTile::simple(HOT_IRON_BLOOM_FLAT, 4, 4),
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::empty(1, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, SMITHING_HAMMER);
    let r = smith_action_to_live_intent(
        &tiles,
        &inp,
        SmithAction::ShortCraft {
            actor: SMITHING_HAMMER,
            target: HOT_IRON_BLOOM_FLAT,
        },
    );
    assert!(r.had_action);
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::UseAt {
            x: 4,
            y: 4,
            target_id: HOT_IRON_BLOOM_FLAT,
            actor_id: SMITHING_HAMMER,
        }
    );
    let forge = closest_forge_from_scan(&tiles, 0, 0);
    assert_eq!(forge.map(|f| f.0), Some(FIRING_FORGE));
}

/// AI-POTTER-L2946: live smith DeferPottery fills pottery counts and expands L2946 crafts.
// Haxe: prepareSmithingTools ~3680 → doPotteryOnFire residual
#[test]
fn smith_defer_pottery_live_expands_l2946_crafts() {
    // Wet nozzle under max → craftItem(296) Fired Nozzle tongs
    let tiles_nozzle = vec![
        ScanTile::simple(FIRING_KILN, 0, 0),
        ScanTile::simple(WET_CLAY_NOZZLE, 1, 0),
        ScanTile::empty(2, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = smith_action_to_live_intent(&tiles_nozzle, &inp, SmithAction::DeferPottery);
    assert!(r.had_action);
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: FIRED_NOZZLE_TONGS
        }
    );

    // Wet bowls (no close-bowl FIX) → residual wet-bowl fire craftItem(283)
    let tiles_bowl = vec![
        ScanTile::simple(FIRING_KILN, 0, 0),
        ScanTile::simple(WET_CLAY_BOWL, 1, 0),
        ScanTile::simple(WET_CLAY_BOWL, 2, 0),
        ScanTile::empty(3, 0, 0, 0),
    ];
    let r2 = smith_action_to_live_intent(&tiles_bowl, &inp, SmithAction::DeferPottery);
    assert!(r2.had_action);
    assert_eq!(
        r2.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: FIRED_BOWL_TONGS
        }
    );

    // Kiln + coal≥3, no residual wet stock → adobe gate closed → DeferPottery seek
    let tiles_empty = vec![
        ScanTile::simple(FIRING_KILN, 0, 0),
        ScanTile::simple(BIG_CHARCOAL_PILE, 1, 0),
        ScanTile::simple(BIG_CHARCOAL_PILE, 2, 0),
        ScanTile::simple(BIG_CHARCOAL_PILE, 3, 0),
        ScanTile::empty(4, 0, 0, 0),
    ];
    let r3 = smith_action_to_live_intent(&tiles_empty, &inp, SmithAction::DeferPottery);
    assert!(r3.had_action);
    assert_eq!(
        r3.intent,
        ShortCraftLiveIntent::SeekOrCraft {
            actor: FIRING_KILN,
            craft_if_needed: false,
        }
    );
}

#[test]
fn baker_defer_pottery_empty_returns_false_like_do_pottery() {
    // Haxe L3266 `return doPottery(1)`: no kiln → pottery abort → baking returns false
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferPottery);
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "empty doPottery(1) must end baking, got {:?}",
        r
    );
}

#[test]
fn baker_defer_pottery_then_cleanup_hungry_skips_cleanup() {
    // Haxe L3376 cleanUp skipped when hungry after doPottery(1) false
    let tiles = vec![
        ScanTile::simple(GOOSEBERRY, 1, 0),
        ScanTile::simple(GOOSEBERRY, 2, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_hungry = true;
    inp.age = 21.0;
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferPotteryThenCleanup);
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "hungry late pottery abort must not cleanUp, got {:?}",
        r
    );
}

#[test]
fn baker_defer_pottery_max_people_one_ignores_baker_assigned() {
    // Assigned baker must not lift nested pottery to doPottery(100)
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = true;
    inp.peer_count = 1.0;
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferPottery);
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "doPottery(1) peer cap must abort, got {:?}",
        r
    );
}

#[test]
fn baker_mutton_hot_oven_use_at() {
    let tiles = vec![
        ScanTile::simple(HOT_OVEN, 7, 8),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, RAW_MUTTON);
    let r = bake_action_to_live_intent(
        &tiles,
        &inp,
        BakeAction::ShortCraft {
            actor: RAW_MUTTON,
            target: HOT_OVEN,
        },
    );
    assert!(r.had_action);
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::UseAt {
            x: 7,
            y: 8,
            target_id: HOT_OVEN,
            actor_id: RAW_MUTTON,
        }
    );
}

#[test]
fn baker_mutton_max_new_actor_counts_transition_new_actor() {
    // Haxe shortCraftOnTarget(569, hotOven, false, 4): CountCloseObjects(570, 30)
    use crate::baker_profession::COOKED_MUTTON;
    use ol_content::{ContentDb, Transition};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    db.transitions.insert(
        (RAW_MUTTON, HOT_OVEN),
        Transition {
            actor_id: RAW_MUTTON,
            target_id: HOT_OVEN,
            new_actor_id: COOKED_MUTTON,
            new_target_id: HOT_OVEN,
            ..Default::default()
        },
    );
    let oven = ScanTile::simple(HOT_OVEN, 7, 8);
    let empty = ScanTile::empty(0, 1, 0, 0);
    let cooked: Vec<_> = (0..4)
        .map(|i| ScanTile::simple(COOKED_MUTTON, i, 0))
        .collect();
    let mut tiles = vec![oven, empty];
    tiles.extend(cooked);
    let mut inp = ProfessionScanInput::basic(0, 0, RAW_MUTTON);
    inp.content = Some(Arc::new(db));
    let r = bake_action_to_live_intent(
        &tiles,
        &inp,
        BakeAction::ShortCraft {
            actor: RAW_MUTTON,
            target: HOT_OVEN,
        },
    );
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "4 cooked newActor must refuse, got {:?}",
        r
    );
    assert_eq!(
        short_craft_scan_new_actor_count(&tiles, &inp, RAW_MUTTON, HOT_OVEN),
        4
    );

    let mut under = vec![
        ScanTile::simple(HOT_OVEN, 7, 8),
        ScanTile::empty(0, 1, 0, 0),
    ];
    for i in 0..3 {
        under.push(ScanTile::simple(COOKED_MUTTON, i, 0));
    }
    let r_ok = bake_action_to_live_intent(
        &under,
        &inp,
        BakeAction::ShortCraft {
            actor: RAW_MUTTON,
            target: HOT_OVEN,
        },
    );
    assert_eq!(
        r_ok.intent,
        ShortCraftLiveIntent::UseAt {
            x: 7,
            y: 8,
            target_id: HOT_OVEN,
            actor_id: RAW_MUTTON,
        }
    );

    // 4 raw nearby is not trans.newActor — still USE
    let mut raws = vec![
        ScanTile::simple(HOT_OVEN, 7, 8),
        ScanTile::empty(0, 1, 0, 0),
    ];
    for i in 0..4 {
        raws.push(ScanTile::simple(RAW_MUTTON, i, 2));
    }
    let r_raw = bake_action_to_live_intent(
        &raws,
        &inp,
        BakeAction::ShortCraft {
            actor: RAW_MUTTON,
            target: HOT_OVEN,
        },
    );
    assert_eq!(
        r_raw.intent,
        ShortCraftLiveIntent::UseAt {
            x: 7,
            y: 8,
            target_id: HOT_OVEN,
            actor_id: RAW_MUTTON,
        }
    );
}

#[test]
fn baker_sauerkraut_and_cooked_turkey_shortcraft_r20() {
    // Haxe L3232 shortCraft(0, 2185, 20); L3242 shortCraft(235, 1241, 20, 1)
    let tiles = vec![
        ScanTile::simple(SAUERKRAUT, 5, 0),
        ScanTile::simple(COOKED_TURKEY, 8, 0),
        ScanTile::simple(SAUERKRAUT, 25, 0), // beyond r=20
    ];
    let inp = ProfessionScanInput::basic(0, 0, CLAY_BOWL);
    let r = bake_action_to_live_intent(
        &tiles,
        &inp,
        BakeAction::ShortCraft {
            actor: CLAY_BOWL,
            target: SAUERKRAUT,
        },
    );
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::UseAt {
            x: 5,
            y: 0,
            target_id: SAUERKRAUT,
            actor_id: CLAY_BOWL,
        }
    );
    let far_only = vec![ScanTile::simple(SAUERKRAUT, 25, 0)];
    let r_far = bake_action_to_live_intent(
        &far_only,
        &inp,
        BakeAction::ShortCraft {
            actor: CLAY_BOWL,
            target: SAUERKRAUT,
        },
    );
    assert!(
        !matches!(
            r_far.intent,
            ShortCraftLiveIntent::UseAt {
                target_id: SAUERKRAUT,
                ..
            }
        ),
        "sauerkraut at dist 25 must not match shortCraft r=20, got {:?}",
        r_far.intent
    );

    let pickup = bake_action_to_live_intent(
        &tiles,
        &ProfessionScanInput::basic(0, 0, 0),
        BakeAction::ShortCraft {
            actor: 0,
            target: COOKED_TURKEY,
        },
    );
    assert_eq!(
        pickup.intent,
        ShortCraftLiveIntent::UseAt {
            x: 8,
            y: 0,
            target_id: COOKED_TURKEY,
            actor_id: 0,
        }
    );
}

#[test]
fn get_closest_object_by_id_quad_and_empty() {
    // Haxe L3462–3479: default dist 30; quad dx²+dy² > dist² rejects
    assert_eq!(GET_CLOSEST_OBJECT_BY_ID_DEFAULT_DIST, 30);
    let far_diag = vec![ScanTile::simple(CLAY_PLATE, 22, 22)];
    assert!(
        get_closest_object_by_id(&far_diag, CLAY_PLATE, 0, 0, 30).is_none(),
        "22²+22²=968 > 900"
    );
    let ok = vec![ScanTile::simple(CLAY_PLATE, 21, 21)];
    let t = get_closest_object_by_id(&ok, CLAY_PLATE, 0, 0, 30).unwrap();
    assert_eq!((t.x, t.y), (21, 21));
    let empties = vec![
        ScanTile::empty(0, 0, 0, 0),
        ScanTile::empty(3, 0, 0, 0),
    ];
    let e = get_closest_object_by_id(&empties, 0, 0, 0, 30).unwrap();
    assert_eq!((e.parent_id, e.x, e.y), (0, 3, 0));
}

#[test]
fn build_intent_ctx_soil_well_home_anchors() {
    let tiles = vec![
        ScanTile::simple(663, 10, 10),
        ScanTile::empty(11, 10, 0, 0),
        ScanTile::empty(0, 1, 0, 0),
        ScanTile::simple(DYING_BUSH, 5, 5),
    ];
    let target = closest_by_parent_id(&tiles, DYING_BUSH, 0, 0, 30);
    let ctx = build_intent_ctx(&tiles, 0, 0, 0, 0, target, None, true);
    assert_eq!((ctx.target_x, ctx.target_y), (5, 5));
    assert_eq!(ctx.empty_near_well_x, Some(11));
}

#[test]
fn farm_profession_scan_tick_assigned_rung_no_panic() {
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 2, 2),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let mut task = FarmTaskState::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::BerryFarmer),
        "ASSIGNED_JOB",
        &mut task,
        true,
        &mut FarmProfessionRuntime::default(),
    );
    let _ = r;
}

#[test]
fn convert_scan_to_all_map_types() {
    let tiles = vec![
        ScanTile::simple(100, 1, 1).with_uses(3),
        ScanTile::empty(2, 2, 0, 0),
    ];
    let farm = farm_map_from_scan(&tiles);
    assert_eq!(farm.len(), 1);
    assert_eq!(farm[0].uses, 3);
    let bake = bake_map_from_scan(&tiles);
    assert_eq!(bake.len(), 1);
    let smith = smith_map_from_scan(&tiles);
    assert_eq!(smith.len(), 1);
    assert_eq!(smith[0].parent_id, 100);
}

#[test]
fn profession_scan_tick_dispatch_farm() {
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 3, 3),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = profession_scan_tick(
        ProfessionScanKind::Farm,
        &tiles,
        &inp,
        "ASSIGNED_JOB",
        Some(FarmProfession::BerryFarmer),
        &mut farm_task,
        true,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    if r.had_action {
        assert!(
            matches!(
                r.intent,
                ShortCraftLiveIntent::UseAt { .. }
                    | ShortCraftLiveIntent::SeekOrCraft { .. }
                    | ShortCraftLiveIntent::DropAt { .. }
                    | ShortCraftLiveIntent::CraftItem { .. }
            ),
            "unexpected intent {:?}",
            r.intent
        );
    }
}

#[test]
fn profession_scan_tick_dispatch_smith_baker_no_panic() {
    let tiles = vec![
        ScanTile::simple(HOT_IRON_BLOOM_FLAT, 2, 2),
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::simple(HOT_OVEN, 4, 4),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime {
        is_last_smith: true,
        is_assigned_smith: true,
        stage: 1.0,
    };
    let mut baker_rt = BakerProfessionRuntime {
        is_last_baker: true,
        is_assigned_baker: true,
        ..Default::default()
    };
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let smith_inp = ProfessionScanInput::basic(0, 0, SMITHING_HAMMER);
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let _ = profession_scan_tick(
        ProfessionScanKind::Smith,
        &tiles,
        &smith_inp,
        "ASSIGNED_JOB",
        None,
        &mut farm_task,
        false,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    let baker_inp = ProfessionScanInput::basic(0, 0, RAW_MUTTON);
    let _ = profession_scan_tick(
        ProfessionScanKind::Baker,
        &tiles,
        &baker_inp,
        "ASSIGNED_JOB",
        None,
        &mut farm_task,
        false,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
}

#[test]
fn closest_empty_respects_not_floored_and_home_clearance() {
    // Bowl of Soil 1137: needs bare ground + not within 6 of home.
    let tiles = vec![
        ScanTile::empty(1, 0, 5, 0),  // floored â" skip for 1137
        ScanTile::empty(2, 0, 0, 0),  // bare but d=2 from home (0,0) < 6 â" skip
        ScanTile::empty(7, 0, 0, 0),  // bare, d=7 from home â" ok
        ScanTile::empty(8, 0, 0, 0),
    ];
    let opts = ClosestEmptyOpts::for_held(1137, 0, 0);
    let e = closest_empty_tile_ex(&tiles, 0, 0, 30, opts).unwrap();
    assert_eq!(e, (7, 0));
    // Without held rules, closest non-self empty is (1,0)
    let e2 = closest_empty_tile(&tiles, 0, 0, 30).unwrap();
    assert_eq!(e2, (1, 0));
}

#[test]
fn has_carrot_seeds_from_scan_threshold() {
    let few = vec![ScanTile::simple(SEEDING_CARROTS, 1, 1)];
    assert!(!has_carrot_seeds_from_scan(&few));
    let enough = vec![
        ScanTile::simple(SEEDING_CARROTS, 1, 1),
        ScanTile::simple(BOWL_OF_CARROT_SEEDS, 2, 2),
    ];
    assert!(has_carrot_seeds_from_scan(&enough));
    assert!(!has_bean_seeds_from_scan(&enough));
    let beans = vec![
        ScanTile::simple(BOWL_OF_DRY_BEANS, 0, 0),
        ScanTile::simple(DRY_BEAN_PLANTS, 1, 0),
    ];
    assert!(has_bean_seeds_from_scan(&beans));
    // Haxe: AiBase.hasPepperSeeds L1376–1380 count > 1
    let pepper_few = vec![ScanTile::simple(HOT_PEPPER, 0, 0)];
    assert!(!has_pepper_seeds_from_scan(&pepper_few));
    let pepper_enough = vec![
        ScanTile::simple(HOT_PEPPER, 0, 0),
        ScanTile::simple(PEPPER_SEED, 1, 0),
    ];
    assert!(has_pepper_seeds_from_scan(&pepper_enough));
    // Haxe: AiBase.hasOnionSeeds L1383–1386 count > 1
    let onion_few = vec![ScanTile::simple(WILD_ONION, 0, 0)];
    assert!(!has_onion_seeds_from_scan(&onion_few));
    let onion_enough = vec![
        ScanTile::simple(WILD_ONION_IN_GROUND, 0, 0),
        ScanTile::simple(ONION, 1, 0),
        ScanTile::simple(RIPE_ONIONS, 2, 0),
    ];
    assert!(has_onion_seeds_from_scan(&onion_enough));
    // Haxe: AiBase.countSeeds cornCount > 2
    let corn_few = vec![
        ScanTile::simple(1115, 0, 0),
        ScanTile::simple(1247, 1, 0),
    ];
    assert!(!has_corn_seeds_from_scan(&corn_few));
    let corn_enough = vec![
        ScanTile::simple(1115, 0, 0),
        ScanTile::simple(1247, 1, 0),
        ScanTile::simple(4106, 2, 0),
    ];
    assert!(has_corn_seeds_from_scan(&corn_enough));
    let flags = count_seeds_from_scan(&corn_enough);
    assert!(flags.has_corn_seeds);
    assert!(!flags.early_return);
}

#[test]
fn empty_near_well_places_drop_within_20() {
    let tiles = vec![
        ScanTile::simple(663, 10, 10),
        ScanTile::empty(12, 10, 0, 0),
        ScanTile::empty(50, 50, 0, 0),
    ];
    let e = empty_near_well(&tiles, 0, 0, 60).unwrap();
    assert_eq!(e, (12, 10));
    let d = scan_chebyshev(10, 10, e.0, e.1);
    assert!(d <= 20);
}

#[test]
fn get_close_well_prefers_deep_663_radius_40() {
    // Haxe L2676–2689 wellIs = [663, 662]; default searchDistance 40
    let tiles = vec![
        ScanTile::simple(662, 3, 0),
        ScanTile::simple(663, 3, 0),
        ScanTile::simple(662, 50, 0),
    ];
    let w = get_close_well(&tiles, 0, 0).unwrap();
    assert_eq!(w.parent_id, 663);
    assert_eq!(GET_CLOSE_WELL_RADIUS, 40);
    assert_eq!(WELL_IDS, [663, 662]);
    // Beyond r=40 is ignored
    let far_only = vec![ScanTile::simple(662, 50, 0)];
    assert!(get_close_well(&far_only, 0, 0).is_none());
}

#[test]
fn short_craft_on_ground_336_no_well_min_dist_5() {
    // Haxe L2701–2704: no well → minDist 5 from home, empty r=20
    let tiles = vec![
        ScanTile::empty(1, 0, 0, 0),
        ScanTile::empty(6, 0, 0, 0),
    ];
    let e = empty_near_well_ex(&tiles, 0, 0, GET_CLOSE_WELL_RADIUS, BASKET_OF_SOIL).unwrap();
    assert_eq!(e, (6, 0));
}

#[test]
fn baker_defer_farm_had_action_expands_or_stages() {
    // Empty map: expand still marks had_action (may chain to sheep/advanced none).
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferFarm);
    assert!(r.had_action);
}

#[test]
fn baker_defer_cleanup_expands_clean_up_bowls_gooseberry() {
    // Haxe cleanUpBowls(253) remaps to gooseberry 31; ≥2 single-use → shortCraft(0, 31)
    let tiles = vec![
        ScanTile::simple(GOOSEBERRY, 1, 0),
        ScanTile::simple(GOOSEBERRY, 2, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    // Haxe: AiBase.cleanUp L1025 age%3==0 required for bowls
    inp.age = 21.0;
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferCleanup);
    assert!(r.had_action);
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                target_id: GOOSEBERRY,
                actor_id: 0,
                ..
            } | ShortCraftLiveIntent::SeekOrCraft {
                actor: GOOSEBERRY,
                ..
            }
        ),
        "expected gooseberry cleanUpBowls live, got {:?}",
        r.intent
    );
}

#[test]
fn baker_defer_seats_cleanup_falls_through_clean_up_bowls() {
    // Tomato seeds already on map → makeSeatsAndCleanUp returns false; Haxe continues to cleanUp.
    // Empty hands so cleanup is not a drop-held of 2828.
    let tiles = vec![
        ScanTile::simple(BOWL_TOMATO_SEEDS, 3, 0),
        ScanTile::simple(GOOSEBERRY, 1, 0),
        ScanTile::simple(GOOSEBERRY, 2, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    // Haxe: AiBase.cleanUp L1025 age%3==0 required for bowls
    inp.age = 21.0;
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferSeatsCleanup);
    assert!(r.had_action);
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                target_id: GOOSEBERRY,
                actor_id: 0,
                ..
            } | ShortCraftLiveIntent::SeekOrCraft {
                actor: GOOSEBERRY,
                ..
            }
        ),
        "expected seats empty-branch cleanUpBowls, got {:?}",
        r.intent
    );
}

#[test]
fn baker_defer_seats_hungry_skips_cleanup() {
    // Haxe makeSeatsAndCleanUp and cleanUp both return false when hungry.
    let tiles = vec![
        ScanTile::simple(GOOSEBERRY, 1, 0),
        ScanTile::simple(GOOSEBERRY, 2, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_hungry = true;
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferSeatsCleanup);
    assert!(r.had_action);
    assert!(
        matches!(r.intent, ShortCraftLiveIntent::None),
        "hungry seats must not fall through to gooseberry cleanUp, got {:?}",
        r.intent
    );
}

#[test]
fn baker_defer_plant_carrots_expands_craft_item() {
    // Low carrot stock + carrot_planter latch via do_plant_carrots(2,10)
    let tiles = vec![
        ScanTile::simple(crate::farmer_profession::DRY_PLANTED_CARROTS, 1, 0),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferPlantCarrots);
    assert!(r.had_action);
    // With planted=1 → stock units = 4; below max 10 but planter may need ≤min.
    // do_plant_carrots sets planter when count≤min(2); stock=4 so planter stays 0 → None
    // unless we seed more scarcity — use zero planted for CraftItem.
    let tiles_scarce = vec![ScanTile::empty(0, 0, 0, 0)];
    let r2 = bake_action_to_live_intent(&tiles_scarce, &inp, BakeAction::DeferPlantCarrots);
    assert!(r2.had_action);
    // First call with scarce: count=0 ≤ min → planter=1 → CraftItem dry planted carrots
    assert!(
        matches!(
            r2.intent,
            ShortCraftLiveIntent::CraftItem {
                object_id: crate::farmer_profession::DRY_PLANTED_CARROTS
            }
        ),
        "expected CraftItem dry carrots, got {:?}",
        r2.intent
    );
}

#[test]
fn smith_new_actor_count_from_scan_non_panic() {
    // Two hammers already on map â' new_actor_count=2 when shortCraft hammer.
    let tiles = vec![
        ScanTile::simple(SMITHING_HAMMER, 1, 0),
        ScanTile::simple(SMITHING_HAMMER, 2, 0),
        ScanTile::simple(HOT_IRON_BLOOM_FLAT, 3, 0),
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, SMITHING_HAMMER);
    let r = smith_action_to_live_intent(
        &tiles,
        &inp,
        SmithAction::ShortCraft {
            actor: SMITHING_HAMMER,
            target: HOT_IRON_BLOOM_FLAT,
        },
    );
    assert!(r.had_action);
    assert!(matches!(
        r.intent,
        ShortCraftLiveIntent::UseAt {
            target_id: HOT_IRON_BLOOM_FLAT,
            ..
        }
    ));
}

#[test]
fn smith_craft_drop_live_counts_and_pickup() {
    // Haxe GetCraftAndDropItemsCloseToObj: CountCloseObjects(forge, id, dist);
    // already enough → false; else pickup outside band; else craftItem.
    let drop = SmithAction::CraftAndDropNearForge {
        object_id: FLAT_ROCK,
        want_count: 2,
    };
    let enough = vec![
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::simple(FLAT_ROCK, 1, 0),
        ScanTile::simple(FLAT_ROCK, 2, 0),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = smith_action_to_live_intent(&enough, &inp, drop);
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "2 flat rocks within dist=5 must skip craft-drop, got {:?}",
        r
    );

    // Rock outside count band (d=8 >= dist=5) → PickupNearForge at object tile
    let far = vec![
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::simple(FLAT_ROCK, 8, 0),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let r_pick = smith_action_to_live_intent(&far, &inp, drop);
    assert_eq!(
        r_pick.intent,
        ShortCraftLiveIntent::PickupNearForge {
            object_id: FLAT_ROCK,
            x: 8,
            y: 0,
        }
    );

    // No rocks → CraftItem
    let none = vec![
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let r_craft = smith_action_to_live_intent(&none, &inp, drop);
    assert_eq!(
        r_craft.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: FLAT_ROCK
        }
    );
}

#[test]
fn smith_craft_drop_live_held_goto_and_empty_drop() {
    let drop = SmithAction::CraftAndDropNearForge {
        object_id: FLAT_ROCK,
        want_count: 2,
    };
    // Held + far from forge → GotoForge
    let tiles = vec![
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let mut far = ProfessionScanInput::basic(20, 0, FLAT_ROCK);
    far.home_x = 0;
    far.home_y = 0;
    let r_goto = smith_action_to_live_intent(&tiles, &far, drop);
    assert!(
        matches!(
            r_goto.intent,
            ShortCraftLiveIntent::GotoForge {
                object_id: FLAT_ROCK,
                forge_x: 0,
                forge_y: 0,
            }
        ),
        "got {:?}",
        r_goto.intent
    );

    // Held + close → dropHeldObject(5, forge) on empty, not ON forge
    let close = ProfessionScanInput::basic(0, 1, FLAT_ROCK);
    let r_drop = smith_action_to_live_intent(&tiles, &close, drop);
    assert!(
        matches!(r_drop.intent, ShortCraftLiveIntent::DropAt { x, y } if !(x == 0 && y == 0)),
        "drop near forge must not be forge tile, got {:?}",
        r_drop.intent
    );
    assert_eq!(r_drop.intent, ShortCraftLiveIntent::DropAt { x: 1, y: 0 });
}

#[test]
fn smith_crucible_craft_drop_uses_dist_ten() {
    // Haxe GetCraftAndDropItemsCloseToObj(forge, 319, 3, 10)
    let drop = SmithAction::CraftAndDropNearForge {
        object_id: UNFORGED_SEALED_CRUCIBLE,
        want_count: 3,
    };
    let tiles = vec![
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::simple(UNFORGED_SEALED_CRUCIBLE, 6, 0),
        ScanTile::simple(UNFORGED_SEALED_CRUCIBLE, 7, 0),
        ScanTile::simple(UNFORGED_SEALED_CRUCIBLE, 8, 0),
        ScanTile::empty(0, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = smith_action_to_live_intent(&tiles, &inp, drop);
    assert!(
        !r.had_action && matches!(r.intent, ShortCraftLiveIntent::None),
        "3 crucibles within dist=10 must skip; dist=5 would still craft. got {:?}",
        r
    );
}

#[test]
fn player_path_chisel_family_uses_sim_cache() {
    // SMITH-CHISEL-PLAYER-CACHE: SimState::new / seed caches extras; ticks clone, no re-scan
    use ol_content::{ContentDb, ObjectDef};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    let mut o = ObjectDef::empty(8888);
    o.description = "Ritual Chisel Stone".into();
    db.objects.insert(8888, o);
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    assert!(
        state.steel_chisel_family.extras.contains(&8888),
        "load-time cache extras, got {:?}",
        state.steel_chisel_family.extras
    );
    assert_eq!(
        chisel_family_extra_from_state(&state),
        state.steel_chisel_family.extras
    );
    {
        let db = Arc::make_mut(&mut state.content);
        if let Some(obj) = db.objects.get_mut(&8888) {
            obj.description.clear();
        }
        let mut n = ObjectDef::empty(7777);
        n.description = "New Chisel".into();
        db.objects.insert(7777, n);
    }
    let extra = chisel_family_extra_from_state(&state);
    assert!(extra.contains(&8888), "player tick clones cache not live scan");
    assert!(
        !extra.contains(&7777),
        "must not re-scan ContentDb each tick"
    );
    crate::seed_craft_graph_from_content(&mut state);
    let extra = chisel_family_extra_from_state(&state);
    assert!(extra.contains(&7777), "re-seed refreshes table");
    assert!(!extra.contains(&8888), "cleared desc dropped on re-seed");
}

#[test]
fn peer_snapshots_count_same_home_sticky() {
    use crate::Player;
    let mut a = Player::new(1, 1, "a@t");
    a.home_x = 5;
    a.home_y = 5;
    a.age = 20.0;
    a.smith_profession.is_last_smith = true;
    let mut b = Player::new(2, 2, "b@t");
    b.home_x = 5;
    b.home_y = 5;
    b.age = 20.0;
    b.smith_profession.is_last_smith = true;
    let mut c = Player::new(3, 3, "c@t");
    c.home_x = 99;
    c.home_y = 99;
    c.age = 20.0;
    c.smith_profession.is_last_smith = true;
    let peers = smith_peers_from_players([&a, &b, &c], 1, 5, 5);
    assert_eq!(peers.len(), 2); // excludes self conn 1
    let n = crate::count_smith_peers_filtered(&peers, 3.0, 120.0);
    assert_eq!(n, 1.0); // only b same home sticky
}

// â"â" NPC-CRAFT-LADDER: rung â' scan plan + job sensors â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"

#[test]
fn age_rotated_to_scan_step_maps_farm_and_baker() {
    let berry = age_rotated_to_scan_step(AgeRotatedJobKind::BerryFarming).unwrap();
    assert_eq!(berry.kind, ProfessionScanKind::Farm);
    assert_eq!(berry.farm_job, Some(FarmProfession::BerryFarmer));
    assert_eq!(berry.rung_label, "AGE_ROTATED_JOB");
    let basic = age_rotated_to_scan_step(AgeRotatedJobKind::BasicFarming).unwrap();
    assert_eq!(basic.farm_job, Some(FarmProfession::BasicFarmer));
    let bake = age_rotated_to_scan_step(AgeRotatedJobKind::Baking).unwrap();
    assert_eq!(bake.kind, ProfessionScanKind::Baker);
    let pottery = age_rotated_to_scan_step(AgeRotatedJobKind::Pottery).unwrap();
    assert_eq!(pottery.kind, ProfessionScanKind::Pottery);
    assert_eq!(pottery.rung_label, "AGE_ROTATED_JOB");
    // AI-SHEPHERD: age-rotated sheep maps to ProfessionScanKind::Shepherd
    let sheep = age_rotated_to_scan_step(AgeRotatedJobKind::SheepHerding).unwrap();
    assert_eq!(sheep.kind, ProfessionScanKind::Shepherd);
    assert_eq!(sheep.rung_label, "AGE_ROTATED_JOB");
}

#[test]
fn age_rotated_to_scan_step_sheep_herding_some_after_wire() {
    let sheep = age_rotated_to_scan_step(AgeRotatedJobKind::SheepHerding);
    assert!(sheep.is_some());
    assert_eq!(sheep.unwrap().kind, ProfessionScanKind::Shepherd);
}

#[test]
fn plan_assigned_job_steps_includes_shepherd_when_last_or_assigned_shepherd() {
    let sticky = ProfessionStickySnapshot {
        shepherd_assigned: true,
        shepherd_last: true,
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_assigned_job_steps(&sticky);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind, ProfessionScanKind::Shepherd);
    assert!(steps[0].is_assigned_job);

    let last_only = ProfessionStickySnapshot {
        shepherd_last: true,
        age: 20.0,
        ..Default::default()
    };
    let steps2 = plan_assigned_job_steps(&last_only);
    assert_eq!(steps2[0].kind, ProfessionScanKind::Shepherd);
}

#[test]
fn job_sensor_flags_from_sticky_assigned_and_age() {
    let sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::BasicFarmer),
        farm_last: Some(FarmProfession::BasicFarmer),
        smith_assigned: false,
        smith_last: false,
        baker_assigned: false,
        baker_last: false,
        pottery_assigned: false,
        pottery_last: false,
        shepherd_assigned: false,
        shepherd_last: false,
        fire_food_assigned: false,
        fire_food_last: false,
        fire_keeper_assigned: false,
        fire_keeper_last: false,
        grave_keeper_assigned: false,
        grave_keeper_last: false,
        hunter_assigned: false,
        hunter_last: false,
        lumberjack_assigned: false,
        lumberjack_last: false,
        collector_assigned: false,
        collector_last: false,
        foodserver_assigned: false,
        foodserver_last: false,
        tailor_assigned: false,
        tailor_last: false,
        age: 20.0,
    };
    let f = job_sensor_flags_from_sticky(&sticky);
    assert!(f.has_assigned_job);
    assert!(f.age_job_pending);
    assert!(!f.critical_craft_pending);

    let smith = ProfessionStickySnapshot {
        smith_last: true,
        age: 25.0,
        ..Default::default()
    };
    let f2 = job_sensor_flags_from_sticky(&smith);
    assert!(!f2.has_assigned_job);
    assert!(f2.critical_craft_pending);

    let baby = ProfessionStickySnapshot {
        age: 2.0,
        ..Default::default()
    };
    assert!(!job_sensor_flags_from_sticky(&baby).age_job_pending);
}

#[test]
fn apply_job_flags_to_live_input_sets_ladder_sensors() {
    let sticky = ProfessionStickySnapshot {
        baker_assigned: true,
        baker_last: true,
        age: 15.0,
        ..Default::default()
    };
    let mut input = crate::LiveSensorInput::default();
    apply_job_flags_to_live_input(&mut input, &sticky);
    assert!(input.has_assigned_job);
    assert!(input.age_job_pending);
    assert!(!input.critical_craft_pending);
}

#[test]
fn plan_assigned_job_prefers_farm_smith_baker() {
    let sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::CarrotFarmer),
        smith_assigned: true,
        baker_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_assigned_job_steps(&sticky);
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].kind, ProfessionScanKind::Farm);
    assert_eq!(steps[0].farm_job, Some(FarmProfession::CarrotFarmer));
    assert_eq!(steps[0].rung_label, "ASSIGNED_JOB");
    assert_eq!(steps[1].kind, ProfessionScanKind::Smith);
    assert_eq!(steps[2].kind, ProfessionScanKind::Baker);
}

#[test]
fn plan_profession_ladder_assigned_and_age_rungs() {
    let sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::BasicFarmer),
        age: 10.0, // jobByAge â' Baking first (round(10/5)=2)
        ..Default::default()
    };
    let assigned = plan_profession_ladder_steps(PriorityRung::AssignedJob, &sticky);
    assert_eq!(assigned.len(), 1);
    assert_eq!(assigned[0].kind, ProfessionScanKind::Farm);

    let age_steps = plan_profession_ladder_steps(PriorityRung::AgeRotatedJob, &sticky);
    // age 10 â' Baking, Pottery(skip), Sheep(skip), Berry, Basic
    assert!(age_steps.iter().any(|s| s.kind == ProfessionScanKind::Baker));
    assert!(age_steps
        .iter()
        .any(|s| s.farm_job == Some(FarmProfession::BerryFarmer)));
    assert!(age_steps
        .iter()
        .any(|s| s.farm_job == Some(FarmProfession::BasicFarmer)));

    let crit = plan_profession_ladder_steps(
        PriorityRung::CriticalCraft,
        &ProfessionStickySnapshot {
            smith_last: true,
            age: 20.0,
            ..Default::default()
        },
    );
    // AI-JOB-SMITH-RESID: early sticky + CRITICAL_CRAFT + HOT_KILN + omelette
    assert_eq!(crit.len(), 4);
    assert_eq!(crit[0].rung_label, "EARLY_STICKY_SMITH");
    assert_eq!(crit[1].rung_label, "CRITICAL_CRAFT");
    assert_eq!(crit[2].rung_label, "HOT_KILN");
    assert_eq!(crit[3].rung_label, COOKED_OMELETTE_RUNG);
    let crit_open = plan_profession_ladder_steps(
        PriorityRung::CriticalCraft,
        &ProfessionStickySnapshot {
            age: 20.0,
            ..Default::default()
        },
    );
    assert_eq!(crit_open.len(), 3);
    assert_eq!(crit_open[0].rung_label, "CRITICAL_CRAFT");
    assert_eq!(crit_open[1].rung_label, "HOT_KILN");
    assert_eq!(crit_open[2].rung_label, COOKED_OMELETTE_RUNG);

    let idle = plan_profession_ladder_steps(PriorityRung::Escape, &sticky);
    assert!(idle.is_empty());
}

#[test]
fn low_priority_work_tail_after_job_by_age_matches_haxe_l805_l818() {
    // Haxe: extra isSheepHerding, isCuttingWood, doSmithing, craftLowPriorityClothing, doAdvancedFarming(1)
    let sticky = ProfessionStickySnapshot {
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_profession_ladder_steps(PriorityRung::LowPriorityWork, &sticky);
    let labels: Vec<&str> = steps.iter().map(|s| s.rung_label).collect();
    assert!(labels.contains(&"SHEEP-WIDE"));
    assert!(labels.contains(&"LOW_PRIORITY_WORK"));
    assert!(labels.contains(&"LOW_PRIORITY_CLOTHING"));
    assert!(labels.contains(&"ADVANCED_FARMING_1"));
    let sheep_wide = steps
        .iter()
        .find(|s| s.rung_label == "SHEEP-WIDE")
        .expect("SHEEP-WIDE");
    assert_eq!(sheep_wide.kind, ProfessionScanKind::Shepherd);
    let smith = steps
        .iter()
        .rev()
        .find(|s| s.kind == ProfessionScanKind::Smith)
        .expect("open doSmithing");
    assert_eq!(smith.rung_label, "LOW_PRIORITY_WORK");
    let farm1 = steps
        .iter()
        .find(|s| s.rung_label == "ADVANCED_FARMING_1")
        .expect("doAdvancedFarming(1)");
    assert_eq!(farm1.farm_job, Some(FarmProfession::AdvancedFarmer));
}

#[test]
fn ladder_profession_scan_assigned_farmer_soil_on_bush_use_at() {
    // Assigned BasicFarmer holding Bowl of Soil near Dying Bush â' UseAt.
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 3, 4),
        ScanTile::empty(1, 1, 0, 0),
        ScanTile::empty(2, 2, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::BerryFarmer),
        farm_last: Some(FarmProfession::BerryFarmer),
        age: 20.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = ladder_profession_scan_tick(
        PriorityRung::AssignedJob,
        &tiles,
        &inp,
        &sticky,
        &mut farm_task,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    // BerryFarmer on dying bush with soil often yields ShortCraft UseAt or Seek/Craft.
    if r.had_action {
        assert!(
            matches!(
                r.intent,
                ShortCraftLiveIntent::UseAt { .. }
                    | ShortCraftLiveIntent::SeekOrCraft { .. }
                    | ShortCraftLiveIntent::DropAt { .. }
                    | ShortCraftLiveIntent::CraftItem { .. }
            ),
            "unexpected intent {:?}",
            r.intent
        );
    }
}

#[test]
fn ladder_profession_scan_smith_hammer_bloom_wire_use() {
    let tiles = vec![
        ScanTile::simple(HOT_IRON_BLOOM_FLAT, 4, 4),
        ScanTile::simple(FIRING_FORGE, 0, 0),
        ScanTile::empty(1, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, SMITHING_HAMMER);
    let sticky = ProfessionStickySnapshot {
        smith_assigned: true,
        smith_last: true,
        age: 20.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime {
        is_last_smith: true,
        is_assigned_smith: true,
        stage: 1.0,
    };
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = ladder_profession_scan_tick(
        PriorityRung::AssignedJob,
        &tiles,
        &inp,
        &sticky,
        &mut farm_task,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    // Smith SM may or may not pick hammer+bloom depending on counts; no panic.
    let _ = r;
    // CriticalCraft early sticky path also plans.
    let steps = plan_profession_ladder_steps(PriorityRung::CriticalCraft, &sticky);
    assert_eq!(steps[0].kind, ProfessionScanKind::Smith);
}

#[test]
fn ladder_age_rotated_sequence_tries_until_action() {
    // Age 0 â' Berry first; soil + dying bush should produce action.
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 2, 2),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, BOWL_OF_SOIL);
    let sticky = ProfessionStickySnapshot {
        age: 0.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = ladder_profession_scan_tick(
        PriorityRung::AgeRotatedJob,
        &tiles,
        &inp,
        &sticky,
        &mut farm_task,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    let _ = r; // action depends on full berry SM; ensure no panic
    let plans = plan_age_rotated_steps(0.0);
    assert_eq!(plans[0].farm_job, Some(FarmProfession::BerryFarmer));
    // Sheep is included in age-rotated plans after AI-SHEPHERD wire
    assert!(plans
        .iter()
        .any(|s| s.kind == ProfessionScanKind::Shepherd));
}

#[test]
fn farm_peer_snapshots_count_by_job() {
    use crate::Player;
    let mut a = Player::new(1, 1, "a@t");
    a.home_x = 0;
    a.home_y = 0;
    a.age = 20.0;
    a.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);
    let mut b = Player::new(2, 2, "b@t");
    b.home_x = 0;
    b.home_y = 0;
    b.age = 20.0;
    b.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);
    let mut c = Player::new(3, 3, "c@t");
    c.home_x = 0;
    c.home_y = 0;
    c.age = 20.0;
    c.farm_profession.last_profession = Some(FarmProfession::BerryFarmer);
    let peers = farm_peers_from_players([&a, &b, &c], 1, 0, 0);
    assert_eq!(peers.len(), 2);
    assert_eq!(
        count_farm_peers_for_job(&peers, FarmProfession::BasicFarmer, 3.0, 120.0),
        1.0
    );
    assert_eq!(
        count_farm_peers_for_job(&peers, FarmProfession::BerryFarmer, 3.0, 120.0),
        1.0
    );
}

#[test]
fn escape_outranks_assigned_job_after_job_flags() {
    // Regression: Escape / PickupFood still outrank AssignedJob after wire.
    let sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::BasicFarmer),
        age: 20.0,
        ..Default::default()
    };
    let mut input = crate::LiveSensorInput {
        food: 15.0,
        food_max: 20.0,
        age: 20.0,
        deadly_animal: Some((5, 5, 4.0)),
        ..Default::default()
    };
    apply_job_flags_to_live_input(&mut input, &sticky);
    assert!(input.has_assigned_job);
    let (rung, goal, _) =
        crate::pick_goal_from_live_sensors(&input, crate::Profession::Farmer, false, false);
    assert_eq!(rung, PriorityRung::Escape);
    assert_eq!(goal, crate::Goal::Flee);

    // Hungry + nearby food â' PickupFood / ConsiderMakeFood over AssignedJob
    input.deadly_animal = None;
    input.food = 2.0;
    input.nearby_food = true;
    input.was_hungry = false;
    apply_job_flags_to_live_input(&mut input, &sticky);
    let (rung, _, _) =
        crate::pick_goal_from_live_sensors(&input, crate::Profession::Farmer, false, false);
    assert!(
        matches!(
            rung,
            PriorityRung::PickupFood
                | PriorityRung::ConsiderMakeFood
                | PriorityRung::Eating
                | PriorityRung::BabyHungryMother
        ),
        "food band should beat AssignedJob, got {:?}",
        rung
    );
}

#[test]
fn build_intent_ctx_held_soil_avoids_close_home() {
    let tiles = vec![
        ScanTile::empty(1, 0, 0, 0), // too close to home for soil
        ScanTile::empty(8, 0, 0, 0),
        ScanTile::simple(663, 20, 0),
        ScanTile::empty(21, 0, 0, 0),
    ];
    let ctx = build_intent_ctx_ex(&tiles, 0, 0, 0, 0, None, None, true, 336);
    // Basket of Soil 336: home clearance + well empty preferred
    assert!(
        scan_chebyshev(0, 0, ctx.empty_drop_x, ctx.empty_drop_y) >= DONT_DROP_CLOSE_HOME_MIN
            || (ctx.empty_drop_x, ctx.empty_drop_y) == (0, 0),
        "drop {:?} should respect home clearance",
        (ctx.empty_drop_x, ctx.empty_drop_y)
    );
    assert_eq!(ctx.empty_near_well_x, Some(21));
}

// â"â" NPC-SCAN-FULL: pottery kind + multi-profession ladder â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"

#[test]
fn age_rotated_to_scan_step_pottery_maps() {
    let p = age_rotated_to_scan_step(AgeRotatedJobKind::Pottery).unwrap();
    assert_eq!(p.kind, ProfessionScanKind::Pottery);
    assert!(!p.is_assigned_job);
    assert!(p.profession_is_sticky);
}

#[test]
fn plan_assigned_job_steps_includes_pottery_when_assigned() {
    let sticky = ProfessionStickySnapshot {
        pottery_assigned: true,
        pottery_last: true,
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_assigned_job_steps(&sticky);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind, ProfessionScanKind::Pottery);
    assert!(steps[0].is_assigned_job);
}

#[test]
fn plan_age_rotated_includes_pottery_and_shepherd() {
    let plans = plan_age_rotated_steps(15.0);
    assert!(plans.iter().any(|s| s.kind == ProfessionScanKind::Pottery));
    assert!(plans.iter().any(|s| s.kind == ProfessionScanKind::Shepherd));
    assert!(plans.len() >= 4);
}

#[test]
fn pottery_profession_scan_tick_stone_on_clay_use_at() {
    use crate::pottery_profession::{ADOBE_KILN, CLAY};
    use crate::STONE;
    let tiles = vec![
        ScanTile::simple(ADOBE_KILN, 0, 0),
        ScanTile::simple(CLAY, 1, 0),
        ScanTile::simple(CLAY, 2, 0),
        ScanTile::simple(CLAY, 1, 1),
        ScanTile::simple(CLAY, 2, 1),
        ScanTile::empty(3, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, STONE);
    inp.profession_is_sticky = true;
    inp.is_assigned_job = true;
    let mut rt = crate::PotterProfessionRuntime {
        is_last_potter: true,
        is_assigned_potter: true,
        stage: 3.0,
    };
    let r = pottery_profession_scan_tick(&tiles, &inp, "ASSIGNED_JOB", &mut rt);
    assert!(r.had_action, "expected pottery action");
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt { .. }
                | ShortCraftLiveIntent::SeekOrCraft { .. }
                | ShortCraftLiveIntent::DropAt { .. }
                | ShortCraftLiveIntent::CraftItem { .. }
        ),
        "unexpected {:?}",
        r.intent
    );
}

#[test]
fn ladder_profession_scan_pottery_assigned_no_panic() {
    use crate::pottery_profession::{ADOBE_KILN, CLAY};
    use crate::STONE;
    let tiles = vec![
        ScanTile::simple(ADOBE_KILN, 0, 0),
        ScanTile::simple(CLAY, 1, 0),
        ScanTile::simple(CLAY, 2, 0),
        ScanTile::empty(1, 1, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, STONE);
    let sticky = ProfessionStickySnapshot {
        pottery_assigned: true,
        pottery_last: true,
        age: 20.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime {
        is_last_potter: true,
        is_assigned_potter: true,
        stage: 3.0,
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = ladder_profession_scan_tick(
        PriorityRung::AssignedJob,
        &tiles,
        &inp,
        &sticky,
        &mut farm_task,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    let _ = r;
    let steps = plan_profession_ladder_steps(PriorityRung::AssignedJob, &sticky);
    assert_eq!(steps[0].kind, ProfessionScanKind::Pottery);
}

#[test]
fn profession_scan_tick_dispatch_pottery() {
    use crate::pottery_profession::{ADOBE_KILN, CLAY};
    use crate::STONE;
    let tiles = vec![
        ScanTile::simple(ADOBE_KILN, 0, 0),
        ScanTile::simple(CLAY, 1, 0),
        ScanTile::empty(2, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, STONE);
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime {
        is_last_potter: true,
        is_assigned_potter: true,
        stage: 3.0,
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let _ = profession_scan_tick(
        ProfessionScanKind::Pottery,
        &tiles,
        &inp,
        "ASSIGNED_JOB",
        None,
        &mut farm_task,
        false,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
}

// â"â" NPC-SCAN-RESID: peers_path_nest â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"

#[test]
fn gather_clay_input_basket_with_clay_from_scan_contain() {
    use crate::pottery_profession::{BASKET, CLAY, CLAY_DEPOSIT};
    // Haxe: GetClosestObjectToPosition(home, 292, 10, â¦, [126])
    let tiles = vec![
        ScanTile::simple(BASKET, 1, 0)
            .with_contains(CLAY)
            .with_contained_count(1)
            .with_num_slots(5),
        ScanTile::simple(CLAY_DEPOSIT, 20, 0),
        ScanTile::empty(2, 0, 0, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert!(
        g.basket_with_clay_near_home,
        "ScanTile clay-in-basket near home must set flag without held cargo"
    );
    assert!(!g.basket_full);
}

#[test]
fn gather_clay_input_empty_basket_near_deposit_r5_and_full() {
    use crate::pottery_profession::{BASKET, CLAY, CLAY_DEPOSIT};
    // Haxe: empty basket near deposit r=5; full when contained_count > 2
    let tiles = vec![
        ScanTile::simple(CLAY_DEPOSIT, 10, 0),
        ScanTile::simple(BASKET, 12, 0)
            .with_num_slots(5)
            .with_contained_count(0),
        ScanTile::simple(BASKET, 1, 1)
            .with_contains(CLAY)
            .with_contained_count(3)
            .with_num_slots(5),
    ];
    let mut inp = ProfessionScanInput::basic(10, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert!(g.empty_basket_near_deposit, "empty basket within r=5 of deposit");
    assert!(
        g.basket_with_clay_near_player || g.basket_full,
        "full clay basket near player"
    );
    assert!(g.basket_full, "contained_count>2 â' basket_full");
}

#[test]
fn pottery_profession_scan_tick_held_basket_full_goto_or_drop() {
    use crate::pottery_profession::{BASKET, CLAY, CLAY_DEPOSIT, ADOBE_KILN};
    // Haxe: held basket contained > 2 â' GotoHome / DropHeld (not SeekOrCraft clay)
    let tiles = vec![
        ScanTile::simple(ADOBE_KILN, 0, 0),
        ScanTile::simple(CLAY_DEPOSIT, 30, 0),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(50, 0, BASKET);
    inp.home_x = 0;
    inp.home_y = 0;
    inp.held_contained = 3;
    inp.profession_is_sticky = true;
    inp.is_assigned_job = true;
    let mut rt = crate::PotterProfessionRuntime {
        is_last_potter: true,
        is_assigned_potter: true,
        stage: 1.0,
    };
    let r = pottery_profession_scan_tick(&tiles, &inp, "ASSIGNED_JOB", &mut rt);
    assert!(r.had_action, "full held basket should act");
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::SeekOrCraft { .. }
                | ShortCraftLiveIntent::DropAt { .. }
                | ShortCraftLiveIntent::UseAt { .. }
                | ShortCraftLiveIntent::Goto { .. }
                | ShortCraftLiveIntent::SelfClothing { .. }
        ),
        "expected GotoHome/DropHeld path, got {:?}",
        r.intent
    );
    // Must not seek clay deposit dig when basket is full cargo
    if let ShortCraftLiveIntent::SeekOrCraft { actor, .. } = r.intent {
        assert_ne!(actor, CLAY, "full basket must not seek clay");
        assert_ne!(actor, CLAY_DEPOSIT, "full basket must not seek deposit dig");
    }
}

#[test]
fn closest_by_parent_contains_prefers_clay_basket() {
    use crate::pottery_profession::{BASKET, CLAY};
    let tiles = vec![
        ScanTile::simple(BASKET, 2, 0).with_num_slots(5), // empty, closer
        ScanTile::simple(BASKET, 4, 0)
            .with_contains(CLAY)
            .with_contained_count(1)
            .with_num_slots(5),
    ];
    // Prefer contain only (no fallback)
    let clay_only = closest_by_parent_contains_ex(&tiles, BASKET, CLAY, 0, 0, 10, false).unwrap();
    assert_eq!((clay_only.x, clay_only.y), (4, 0));
    // With fallback: still prefer clay match first
    let with_fb = closest_by_parent_contains(&tiles, BASKET, CLAY, 0, 0, 10).unwrap();
    assert_eq!((with_fb.x, with_fb.y), (4, 0));
}

// â"â" AI-POTTER-NEST: pottery_basket residuals â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"â"

#[test]
fn scan_tile_contains_parent_any_slot_not_only_first() {
    use crate::pottery_profession::{BASKET, CLAY};
    // Haxe: ObjectHelper.contains â" non-clay first slot then clay still matches
    let stone = 33;
    let t = ScanTile::simple(BASKET, 0, 0)
        .with_contains_list(&[stone, CLAY])
        .with_contained_count(2)
        .with_num_slots(5);
    assert_eq!(t.contains_id, stone);
    assert!(t.contains_parent(CLAY), "any-slot clay must match");
    assert!(t.contains_parent(stone));
    assert!(!t.contains_parent(999));
    let found = closest_by_parent_contains_ex(&[t], BASKET, CLAY, 0, 0, 5, false);
    assert!(found.is_some());
}

#[test]
fn gather_clay_input_basket_any_slot_clay_near_home() {
    use crate::pottery_profession::{BASKET, CLAY, CLAY_DEPOSIT};
    // Regression: non-clay first nested id must still set basket_with_clay_near_home
    let tiles = vec![
        ScanTile::simple(BASKET, 1, 0)
            .with_contains_list(&[33, CLAY])
            .with_contained_count(2)
            .with_num_slots(5),
        ScanTile::simple(CLAY_DEPOSIT, 20, 0),
    ];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert!(
        g.basket_with_clay_near_home,
        "any-slot clay-in-basket near home"
    );
}

#[test]
fn gather_clay_input_kiln_remaps_home_for_basket_radius() {
    use crate::pottery_profession::{ADOBE_KILN, BASKET, CLAY, CLAY_DEPOSIT};
    // Haxe: if kiln != null home = kiln; basket with clay r=10 from kiln
    // Basket near kiln (not original home 0,0) should count as near home
    let tiles = vec![
        ScanTile::simple(ADOBE_KILN, 8, 0),
        ScanTile::simple(BASKET, 10, 0)
            .with_contains(CLAY)
            .with_contained_count(1)
            .with_num_slots(5),
        ScanTile::simple(CLAY_DEPOSIT, 30, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert_eq!((g.home_x, g.home_y), (8, 0), "kiln becomes home for gather");
    assert!(
        g.basket_with_clay_near_home,
        "basket within r=10 of kiln counts near home"
    );
}

#[test]
fn gather_clay_input_firing_kiln_remaps_home_before_wood() {
    use crate::pottery_profession::{
        FIRING_ADOBE_KILN, WOOD_FILLED_KILN, BASKET, CLAY, CLAY_DEPOSIT,
    };
    // Haxe helper kiln is 282 first, then 281, then 238 — gather remaps home to that kiln
    let tiles = vec![
        ScanTile::simple(WOOD_FILLED_KILN, 8, 0),
        ScanTile::simple(FIRING_ADOBE_KILN, 12, 0),
        ScanTile::simple(BASKET, 14, 0)
            .with_contains(CLAY)
            .with_contained_count(1)
            .with_num_slots(5),
        ScanTile::simple(CLAY_DEPOSIT, 30, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert_eq!((g.home_x, g.home_y), (12, 0), "firing kiln wins over wood-filled");
    assert!(
        g.basket_with_clay_near_home,
        "basket within r=10 of firing kiln counts near home"
    );
}

#[test]
fn gather_clay_full_basket_near_deposit_only_when_player_far() {
    use crate::pottery_profession::{BASKET, CLAY_DEPOSIT, gather_clay, GatherClayInput};
    // Full basket at deposit, player far, no clay-near-player flags â' still PickupBasket
    let tiles = vec![
        ScanTile::simple(CLAY_DEPOSIT, 50, 0),
        ScanTile::simple(BASKET, 51, 0)
            .with_contained_count(3)
            .with_num_slots(5),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert!(g.full_basket_near_deposit);
    assert!(g.basket_full);
    assert!(!g.basket_with_clay_near_player);
    assert!(!g.empty_basket_near_deposit);
    assert_eq!(
        gather_clay(&g),
        crate::pottery_profession::PotteryAction::PickupBasket
    );
    // pure path without scan flags
    let pure = GatherClayInput {
        player_x: 0,
        player_y: 0,
        home_x: 0,
        home_y: 0,
        has_clay_deposit: true,
        deposit_x: 50,
        deposit_y: 0,
        basket_full: true,
        full_basket_near_deposit: true,
        ..Default::default()
    };
    assert_eq!(
        gather_clay(&pure),
        crate::pottery_profession::PotteryAction::PickupBasket
    );
}

#[test]
fn held_contains_clay_from_helper_nest() {
    use crate::pottery_profession::{BASKET, CLAY};
    use ol_world::NestedHelper;
    // Haxe: heldObject.contains([126]) â" basket with clay nest
    let with_clay = NestedHelper::from_wire(BASKET, &[CLAY, 33]);
    assert!(held_contains_clay(BASKET, Some(&with_clay)));
    assert!(held_nest_contains_parent(Some(&with_clay), CLAY));
    let only_stone = NestedHelper::from_wire(BASKET, &[33]);
    assert!(!held_contains_clay(BASKET, Some(&only_stone)));
    assert!(held_contains_clay(CLAY, None));
    assert!(!held_contains_clay(BASKET, None));
}

#[test]
fn pottery_drop_held_empty_basket_deposit_staging_max_dist_0() {
    use crate::pottery_profession::{BASKET, CLAY_DEPOSIT, empty_basket_drop_is_deposit_staging};
    // Haxe: dropHeldObject(0) empty basket adjacent deposit → feet, not kiln walk
    assert!(empty_basket_drop_is_deposit_staging(BASKET, 0, true));
    let tiles = vec![
        ScanTile::simple(CLAY_DEPOSIT, 5, 0),
        ScanTile::empty(5, 1, 0, 0),
        ScanTile::empty(4, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(5, 0, BASKET);
    inp.home_x = 0;
    inp.home_y = 0;
    inp.held_contained = 0;
    inp.held_contains_clay = false;
    let r = pottery_action_to_live_intent(
        &tiles,
        &inp,
        crate::pottery_profession::PotteryAction::DropHeld {
            allow_piles: false,
            max_distance_to_home: 0,
        },
    );
    assert!(r.had_action);
    // Staging maxDist 0 → drop close to player (feet), not Seek/Goto kiln far away
    match r.intent {
        ShortCraftLiveIntent::DropAt { x, y } => {
            let d = scan_chebyshev(5, 0, x, y);
            assert!(d <= 2, "deposit staging drop near feet, got ({x},{y}) d={d}");
        }
        ShortCraftLiveIntent::UseAt { x, y, .. } => {
            let d = scan_chebyshev(5, 0, x, y);
            assert!(d <= 2, "staging use-drop near feet, got ({x},{y}) d={d}");
        }
        other => panic!("expected feet DropAt/UseAt staging, got {other:?}"),
    }
}

#[test]
fn empty_basket_at_home_live_is_drop_extract() {
    use crate::pottery_profession::{
        empty_basket_at_home_is_drop_extract, BASKET, CLAY, EMPTY_BASKET_HOME_SEARCH_RADIUS,
    };
    // Haxe L3013: dropIsAUse=false + dropTarget=basket → DropAt extract, not USE pickup
    assert!(empty_basket_at_home_is_drop_extract(0));
    assert_eq!(EMPTY_BASKET_HOME_SEARCH_RADIUS, 10);
    let tiles = vec![
        ScanTile::simple(BASKET, 3, 0)
            .with_num_slots(5)
            .with_contains(CLAY)
            .with_contained_count(2),
        ScanTile::simple(BASKET, 50, 0)
            .with_num_slots(5)
            .with_contains(CLAY)
            .with_contained_count(3),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let r = pottery_action_to_live_intent(
        &tiles,
        &inp,
        crate::pottery_profession::PotteryAction::EmptyBasketAtHome,
    );
    assert!(r.had_action);
    match r.intent {
        ShortCraftLiveIntent::DropAt { x, y } => {
            assert_eq!((x, y), (3, 0), "prefer home-near clay basket DropAt extract");
        }
        other => panic!("expected DropAt extract, got {other:?}"),
    }
}

#[test]
fn pottery_action_use_on_basket_prefers_deposit_adjacent() {
    use crate::pottery_profession::{BASKET, CLAY, CLAY_DEPOSIT};
    // Haxe: reuse deposit-adjacent basket over distant any-basket
    let tiles = vec![
        ScanTile::simple(CLAY_DEPOSIT, 10, 0),
        ScanTile::simple(BASKET, 11, 0)
            .with_num_slots(5)
            .with_contained_count(0),
        ScanTile::simple(BASKET, 0, 5)
            .with_num_slots(5)
            .with_contained_count(0),
    ];
    let mut inp = ProfessionScanInput::basic(10, 0, CLAY);
    inp.home_x = 0;
    inp.home_y = 0;
    inp.held_contains_clay = true;
    let r = pottery_action_to_live_intent(
        &tiles,
        &inp,
        crate::pottery_profession::PotteryAction::UseOnBasket,
    );
    assert!(r.had_action);
    match r.intent {
        ShortCraftLiveIntent::UseAt { x, y, target_id, .. } => {
            assert_eq!(target_id, BASKET);
            assert_eq!((x, y), (11, 0), "prefer deposit-adjacent basket");
        }
        other => panic!("expected UseAt basket, got {other:?}"),
    }
}

#[test]
fn pottery_action_pickup_loose_clay_is_drop_at_r5() {
    use crate::pottery_profession::{CLAY, PotteryAction};
    // Haxe L3066–3070: GetClosest clay r=5, dropIsAUse=false dropTarget=clay
    let tiles = vec![
        ScanTile::simple(CLAY, 10, 0),
        ScanTile::simple(CLAY, 4, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let r = pottery_action_to_live_intent(&tiles, &inp, PotteryAction::PickupLooseClay);
    assert!(r.had_action);
    match r.intent {
        ShortCraftLiveIntent::DropAt { x, y } => {
            assert_eq!((x, y), (4, 0), "loose clay only within r=5");
        }
        other => panic!("expected DropAt clay, got {other:?}"),
    }
}

#[test]
fn pottery_action_pickup_basket_uses_deposit_r5_when_player_far() {
    use crate::pottery_profession::{BASKET, CLAY_DEPOSIT, PotteryAction};
    let tiles = vec![
        ScanTile::simple(CLAY_DEPOSIT, 50, 0),
        ScanTile::simple(BASKET, 51, 0)
            .with_num_slots(5)
            .with_contained_count(3),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let r = pottery_action_to_live_intent(&tiles, &inp, PotteryAction::PickupBasket);
    assert!(r.had_action);
    match r.intent {
        ShortCraftLiveIntent::UseAt { x, y, target_id, .. } => {
            assert_eq!(target_id, BASKET);
            assert_eq!((x, y), (51, 0), "full basket at deposit r=5");
        }
        other => panic!("expected UseAt deposit basket, got {other:?}"),
    }
}

#[test]
fn gather_clay_input_remote_deposit_in_tiles() {
    use crate::pottery_profession::CLAY_DEPOSIT;
    // When tiles include remote deposit (player-centered r=80 merge), has_clay_deposit set
    let tiles = vec![ScanTile::simple(CLAY_DEPOSIT, 70, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 0;
    inp.home_y = 0;
    let g = gather_clay_input_from_scan(&tiles, &inp, 0);
    assert!(g.has_clay_deposit);
    assert_eq!((g.deposit_x, g.deposit_y), (70, 0));
}

#[test]
fn merge_scan_tiles_dedupes_xy() {
    let a = vec![
        ScanTile::simple(1, 0, 0),
        ScanTile::simple(2, 1, 0),
    ];
    let b = vec![
        ScanTile::simple(99, 0, 0), // same xy as first â" primary wins
        ScanTile::simple(3, 2, 0),
    ];
    let m = merge_scan_tiles(&a, &b);
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].parent_id, 1);
    assert!(m.iter().any(|t| t.parent_id == 3));
    assert!(!m.iter().any(|t| t.parent_id == 99));
}

#[test]
fn smith_peers_wound_and_follow_excluded() {
    use crate::Player;
    use std::collections::HashMap;

    let mut a = Player::new(1, 1, "a@t");
    a.home_x = 5;
    a.home_y = 5;
    a.age = 20.0;
    a.smith_profession.is_last_smith = true;

    let mut b = Player::new(2, 2, "b@t");
    b.home_x = 5;
    b.home_y = 5;
    b.age = 20.0;
    b.food = 10.0;
    b.smith_profession.is_last_smith = true;
    // Heavy wound held: is_wounded_held(true) when not hidden light wound
    b.held_id = 201;
    b.held_helper = Some(ol_world::NestedHelper::id_only(201));

    let mut c = Player::new(3, 3, "c@t");
    c.home_x = 5;
    c.home_y = 5;
    c.age = 20.0;
    c.food = 10.0;
    c.smith_profession.is_last_smith = true;

    let mut following = HashMap::new();
    following.insert(c.p_id, a.p_id); // c follows a â' playerToFollow

    let is_wounded = |p: &Player| p.is_wounded_held(true);
    let peers = smith_peers_from_players_ex(
        [&a, &b, &c],
        1,
        5,
        5,
        Some(&following),
        Some(&is_wounded),
    );
    assert_eq!(peers.len(), 2); // b + c (excludes self a)
    let n = crate::count_smith_peers_filtered(&peers, 3.0, 120.0);
    assert_eq!(
        n, 0.0,
        "wounded peer and following peer both excluded from count"
    );

    // Healthy peer alone counts
    b.held_id = 0;
    b.held_helper = None;
    following.clear();
    let peers2 = smith_peers_from_players_ex(
        [&a, &b],
        1,
        5,
        5,
        Some(&following),
        Some(&is_wounded),
    );
    assert_eq!(crate::count_smith_peers_filtered(&peers2, 3.0, 120.0), 1.0);
}

#[test]
fn potter_peers_self_exclude_same_home_last() {
    use crate::Player;
    let mut a = Player::new(1, 1, "a@t");
    a.home_x = 0;
    a.home_y = 0;
    a.age = 20.0;
    a.food = 5.0;
    a.pottery_profession.is_last_potter = true;
    let mut b = Player::new(2, 2, "b@t");
    b.home_x = 0;
    b.home_y = 0;
    b.age = 20.0;
    b.food = 5.0;
    b.pottery_profession.is_last_potter = true;
    let peers = potter_peers_from_players([&a, &b], 1, 0, 0);
    assert_eq!(peers.len(), 1);
    assert_eq!(crate::count_potter_peers_filtered(&peers, 3.0, 120.0), 1.0);
}

#[test]
fn filter_scan_tiles_path_skips_not_reachable() {
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 1, 0),
        ScanTile::simple(DYING_BUSH, 3, 0),
        ScanTile::simple(DYING_BUSH, 5, 0),
    ];
    let mut filters = ProfessionPathFilters::new();
    filters.mark_not_reachable(1, 0);
    filters.mark_hostile_path(3, 0);
    let kept = filter_scan_tiles_path(&tiles, &filters);
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].x, kept[0].y), (5, 0));

    let closest = closest_by_parent_id_path(&tiles, DYING_BUSH, 0, 0, 10, Some(&filters)).unwrap();
    assert_eq!((closest.x, closest.y), (5, 0));

    // Without filter, closest is (1,0)
    let raw = closest_by_parent_id(&tiles, DYING_BUSH, 0, 0, 10).unwrap();
    assert_eq!((raw.x, raw.y), (1, 0));
}

#[test]
fn apply_profession_scan_target_reachable_false_suppresses_use_at() {
    // shortCraft UseOnTarget with target_reachable=false â' None (not UseAt)
    use crate::short_craft_intent::{short_craft_apply_to_live_intent, ShortCraftIntentCtx};
    use crate::farmer_profession::ShortCraftApply;

    let mut ctx = ShortCraftIntentCtx::at_target(5, 5);
    ctx.target_reachable = false;
    let intent = short_craft_apply_to_live_intent(
        ShortCraftApply::UseOnTarget {
            actor: 33,
            target: DYING_BUSH,
        },
        &ctx,
    );
    assert!(
        matches!(intent, ShortCraftLiveIntent::None),
        "unreachable target must not UseAt, got {:?}",
        intent
    );

    // Farm scan tick with target_reachable=false should not emit UseAt
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 1, 0),
        ScanTile::empty(2, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.target_reachable = false;
    inp.profession_is_sticky = true;
    inp.is_assigned_job = true;
    let mut farm_task = FarmTaskState::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::BerryFarmer),
        "ASSIGNED_JOB",
        &mut farm_task,
        true,
        &mut FarmProfessionRuntime::default(),
    );
    if r.had_action {
        assert!(
            !matches!(r.intent, ShortCraftLiveIntent::UseAt { .. }),
            "path-blocked target_reachable=false must not UseAt, got {:?}",
            r.intent
        );
    }
}

#[test]
fn farm_peers_wound_follow_food_age_filters() {
    use crate::Player;
    use std::collections::HashMap;

    let mut a = Player::new(1, 1, "a@t");
    a.home_x = 0;
    a.home_y = 0;
    a.age = 20.0;
    a.food = 5.0;
    a.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);

    let mut b = Player::new(2, 2, "b@t");
    b.home_x = 0;
    b.home_y = 0;
    b.age = 20.0;
    b.food = -1.0; // food_store < 0 excluded
    b.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);

    let mut c = Player::new(3, 3, "c@t");
    c.home_x = 0;
    c.home_y = 0;
    c.age = 119.0; // > MaxAge-2 with max=120
    c.food = 5.0;
    c.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);

    let mut d = Player::new(4, 4, "d@t");
    d.home_x = 0;
    d.home_y = 0;
    d.age = 20.0;
    d.food = 5.0;
    d.farm_profession.last_profession = Some(FarmProfession::BasicFarmer);

    let mut following = HashMap::new();
    following.insert(d.p_id, a.p_id);

    let peers = farm_peers_from_players_ex(
        [&a, &b, &c, &d],
        1,
        0,
        0,
        Some(&following),
        None,
    );
    // b food, c age, d follow â' none of them count; only would be a but self excluded
    assert_eq!(
        count_farm_peers_for_job(&peers, FarmProfession::BasicFarmer, 3.0, 120.0),
        0.0
    );

    // Healthy sticky peer counts
    b.food = 5.0;
    following.clear();
    let peers2 = farm_peers_from_players_ex([&a, &b], 1, 0, 0, Some(&following), None);
    assert_eq!(
        count_farm_peers_for_job(&peers2, FarmProfession::BasicFarmer, 3.0, 120.0),
        1.0
    );
}

#[test]
fn path_reach_maps_filter_and_cleanup() {
    // PATH-REACH: timed maps â' ProfessionPathFilters â' scan filter
    let mut maps = crate::AiPathReachMaps::new();
    maps.add_not_reachable(1, 0, 90.0);
    maps.add_hostile_path(3, 0, 20.0);
    let mut global = std::collections::HashMap::new();
    crate::add_blocked_by_ai(&mut global, 2, 0, 5.0);
    let filters = path_filters_from_player(&maps, &global);
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 1, 0),
        ScanTile::simple(DYING_BUSH, 2, 0),
        ScanTile::simple(DYING_BUSH, 3, 0),
        ScanTile::simple(DYING_BUSH, 5, 0),
    ];
    let kept = apply_path_filters_to_tiles(&tiles, &filters);
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].x, kept[0].y), (5, 0));

    maps.cleanup(100.0);
    assert!(maps.is_empty());
    crate::cleanup_blocked_by_ai(&mut global, 10.0);
    assert!(global.is_empty());
}

#[test]
fn mark_not_reachable_blocks_subsequent_filter() {
    let mut maps = crate::AiPathReachMaps::new();
    crate::mark_not_reachable_on_player(&mut maps, 7, 8, crate::NOT_REACHABLE_DEFAULT_SECS);
    assert!(maps.is_personal_not_reachable(7, 8));
    let f = path_filters_from_player(&maps, &std::collections::HashMap::new());
    assert!(!f.target_reachable(7, 8));
}

#[test]
fn smart_drop_held_profession_ex_busy_moving_wait() {
    // PREFER-SHORT-WAIT: profession DropHeld with is_moving -> Wait
    // Clay bowl 235 is oven-near; home (0,0), player (20,0), moving => BusyMoving
    let clay_bowl = 235; // CLAY_BOWL
    let hot_oven = 250; // HOT_ADOBE_OVEN
    let tiles = vec![
        ScanTile::simple(hot_oven, 0, 0),
        ScanTile::empty(1, 0, 0, 0),
    ];
    let intent = crate::smart_drop_held_profession_ex(
        &tiles,
        clay_bowl,
        1,
        20,
        0,
        0,
        0,
        20.0,
        false,
        40.0,
        false,
        true, // is_moving
        &[0; 6],
        &[0; 6],
    );
    assert_eq!(
        intent,
        ShortCraftLiveIntent::Wait,
        "moving dropOnStart must Wait, got {intent:?}"
    );
    assert!(crate::drop_held_live_intent_actionable(intent));
    assert!(crate::live_intent_is_wait(intent));
}

/// DROP-HELD-QUIVER: profession DropHeld storeInQuiver from clothing snapshot.
// Haxe: dropHeldObject → storeInQuiver getClothingById Empty Arrow Quiver 874
#[test]
fn smart_drop_held_profession_stores_yew_bow_in_clothing_quiver() {
    let yew_bow = 151;
    let empty_quiver = 874;
    let tiles = vec![ScanTile::empty(1, 0, 0, 0)];
    let mut clothing = [0i32; 6];
    clothing[5] = empty_quiver;
    let intent = crate::smart_drop_held_profession_ex(
        &tiles,
        yew_bow,
        1,
        0,
        0,
        0,
        0,
        20.0,
        false,
        40.0,
        false,
        false,
        &clothing,
        &[0; 6],
    );
    assert_eq!(
        intent,
        ShortCraftLiveIntent::SelfClothing { slot: 5 },
        "Yew Bow + empty quiver clothing → self(0,0,5), got {intent:?}"
    );
}

#[test]
fn smart_drop_held_profession_empty_clothing_does_not_store_bow() {
    let yew_bow = 151;
    let tiles = vec![ScanTile::empty(1, 0, 0, 0)];
    let intent = crate::smart_drop_held_profession_ex(
        &tiles,
        yew_bow,
        1,
        0,
        0,
        0,
        0,
        20.0,
        false,
        40.0,
        false,
        false,
        &[0; 6],
        &[0; 6],
    );
    assert_ne!(
        intent,
        ShortCraftLiveIntent::SelfClothing { slot: 5 },
        "no quiver clothing must not SELF slot 5, got {intent:?}"
    );
}

#[test]
fn profession_scan_input_clothing_snapshot_feeds_quiver() {
    let mut inp = ProfessionScanInput::basic(0, 0, 151);
    inp.clothing[5] = 874;
    let tiles = vec![ScanTile::empty(1, 0, 0, 0)];
    let intent = crate::smart_drop_held_profession_ex(
        &tiles,
        inp.held_id,
        inp.held_uses,
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.food_store,
        false,
        40.0,
        false,
        inp.is_moving,
        &inp.clothing,
        &inp.clothing_uses,
    );
    assert_eq!(intent, ShortCraftLiveIntent::SelfClothing { slot: 5 });
}

#[test]
fn ladder_wait_terminal_helpers() {
    // Haxe: isMoving return true holds tick - Wait is not wire, is wait/actionable
    assert!(!live_intent_is_wire(ShortCraftLiveIntent::Wait));
    assert!(crate::live_intent_is_wait(ShortCraftLiveIntent::Wait));
    assert!(crate::drop_held_live_intent_actionable(ShortCraftLiveIntent::Wait));
    let mut inp = ProfessionScanInput::basic(20, 0, 235);
    inp.is_moving = true;
    assert!(inp.is_moving);
}

// ── AI-FIREFOOD-RUNG: FireFood assigned/last makeFireFood(100) ───────────────

#[test]
fn plan_assigned_job_steps_includes_fire_food_when_assigned_or_last() {
    let sticky = ProfessionStickySnapshot {
        fire_food_assigned: true,
        fire_food_last: true,
        fire_keeper_assigned: false,
        fire_keeper_last: false,
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_assigned_job_steps(&sticky);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].kind, ProfessionScanKind::FireFood);
    assert!(steps[0].is_assigned_job);
    assert_eq!(steps[0].rung_label, "ASSIGNED_JOB");

    let last_only = ProfessionStickySnapshot {
        fire_food_last: true,
        fire_keeper_assigned: false,
        fire_keeper_last: false,
        age: 20.0,
        ..Default::default()
    };
    let steps2 = plan_assigned_job_steps(&last_only);
    assert_eq!(steps2[0].kind, ProfessionScanKind::FireFood);
    assert!(steps2[0].is_assigned_job);
}

#[test]
fn job_sensor_flags_fire_food_assigned() {
    let sticky = ProfessionStickySnapshot {
        fire_food_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let f = job_sensor_flags_from_sticky(&sticky);
    assert!(f.has_assigned_job);
    assert!(sticky.has_sticky_profession());
}

#[test]
fn fire_food_profession_scan_tick_assigned_cooks_mutton_on_coals() {
    use crate::baker_profession::RAW_MUTTON;
    use crate::HOT_COALS;
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 1, 0),
        ScanTile::simple(crate::FIRE, 2, 0),
        ScanTile::simple(RAW_MUTTON, 0, 1),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.profession_is_sticky = true;
    inp.is_assigned_job = true;
    let mut rt = crate::FireFoodProfessionRuntime {
        is_last_fire_food: true,
        is_assigned_fire_food: true,
        weight: 1.0,
    };
    let r = fire_food_profession_scan_tick(&tiles, &inp, "ASSIGNED_JOB", &mut rt);
    assert!(r.had_action, "expected fire-food action {:?}", r.intent);
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt { .. }
                | ShortCraftLiveIntent::SeekOrCraft { .. }
                | ShortCraftLiveIntent::CraftItem { .. }
                | ShortCraftLiveIntent::DropAt { .. }
        ),
        "unexpected {:?}",
        r.intent
    );
    assert!(rt.is_last_fire_food);
}

#[test]
fn ladder_profession_scan_fire_food_assigned_no_panic() {
    use crate::baker_profession::RAW_MUTTON;
    use crate::HOT_COALS;
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 1, 0),
        ScanTile::simple(crate::FIRE, 2, 0),
        ScanTile::simple(RAW_MUTTON, 0, 1),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = true;
    inp.profession_is_sticky = true;
    let sticky = ProfessionStickySnapshot {
        fire_food_assigned: true,
        fire_food_last: true,
        fire_keeper_assigned: false,
        fire_keeper_last: false,
        age: 20.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut farm_rt = FarmProfessionRuntime::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime {
        is_last_fire_food: true,
        is_assigned_fire_food: true,
        weight: 1.0,
    };
    let r = ladder_profession_scan_tick(
        PriorityRung::AssignedJob,
        &tiles,
        &inp,
        &sticky,
        &mut farm_task,
        &mut farm_rt,
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    let steps = plan_profession_ladder_steps(PriorityRung::AssignedJob, &sticky);
    assert_eq!(steps[0].kind, ProfessionScanKind::FireFood);
    assert!(r.had_action, "fire-food assigned ladder should act {:?}", r.intent);
}

#[test]
fn profession_scan_tick_dispatch_fire_food() {
    use crate::baker_profession::RAW_MUTTON;
    use crate::HOT_COALS;
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 1, 0),
        ScanTile::simple(crate::FIRE, 2, 0),
        ScanTile::simple(RAW_MUTTON, 0, 1),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = true;
    inp.profession_is_sticky = true;
    let mut farm_task = FarmTaskState::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime {
        is_last_fire_food: true,
        is_assigned_fire_food: true,
        weight: 1.0,
    };
    let r = profession_scan_tick(
        ProfessionScanKind::FireFood,
        &tiles,
        &inp,
        "ASSIGNED_JOB",
        None,
        &mut farm_task,
        false,
        &mut FarmProfessionRuntime::default(),
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut crate::FireKeeperProfessionRuntime::default(),
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    assert!(r.had_action, "dispatch FireFood should act");
}

#[test]
fn from_runtimes_ex_includes_fire_food_sticky() {
    let farm = FarmProfessionRuntime::default();
    let smith = SmithProfessionRuntime::default();
    let baker = BakerProfessionRuntime::default();
    let fire = crate::FireFoodProfessionRuntime {
        is_assigned_fire_food: true,
        is_last_fire_food: true,
        weight: 1.0,
    };
    let sticky = ProfessionStickySnapshot::from_runtimes_ex(
        &farm, &smith, &baker, None, None, Some(&fire), None, None, None, None, None, None, 25.0,
    );
    assert!(sticky.fire_food_assigned);
    assert!(sticky.fire_food_last);
    assert!(sticky.has_assigned_job());
}

// ── AI-HANDLING-FIRE residuals: temp/hungry plan + late makeFireFood + winter ─

#[test]
fn plan_temperature_and_consider_make_food_emit_handling_fire() {
    let sticky = ProfessionStickySnapshot {
        age: 20.0,
        ..Default::default()
    };
    let temp = plan_profession_ladder_steps(PriorityRung::Temperature, &sticky);
    assert_eq!(temp.len(), 1);
    assert_eq!(temp[0].kind, ProfessionScanKind::HandlingFire);
    assert_eq!(temp[0].rung_label, "TEMPERATURE");
    assert_eq!(
        crate::handling_fire_max_for_dispatch(false, temp[0].rung_label),
        crate::HANDLING_FIRE_TEMP_MAX
    );

    let mid = plan_profession_ladder_steps(PriorityRung::MidPriorityTasks, &sticky);
    assert_eq!(mid[0].kind, ProfessionScanKind::Farm);
    assert_eq!(mid[0].rung_label, PULL_CARROT_ROW_RUNG);
    assert_eq!(mid[0].farm_job, None);
    assert!(!mid[0].is_assigned_job);
    assert_eq!(mid[1].kind, ProfessionScanKind::Farm);
    assert_eq!(mid[1].rung_label, FILL_BERRY_HELD_RUNG);
    assert_eq!(mid[2].kind, ProfessionScanKind::Farm);
    assert_eq!(mid[2].rung_label, FILL_BEAN_HELD_RUNG);
    assert_eq!(mid[3].kind, ProfessionScanKind::HandlingFire);
    assert_eq!(mid[4].kind, ProfessionScanKind::Farm);
    assert_eq!(mid[4].rung_label, crate::knife_stuff::KNIFE_STUFF_RUNG);
    assert_eq!(mid[4].farm_job, None);
    assert!(!mid[4].is_assigned_job);
    assert_eq!(mid[5].kind, ProfessionScanKind::Shepherd);
    assert_eq!(mid[5].rung_label, crate::shepherd_profession::FEED_LAMBS_CALFS_RUNG);
    assert_eq!(mid[6].kind, ProfessionScanKind::Hunting);
    assert_eq!(mid[7].kind, ProfessionScanKind::HandlingGraves);
    assert_eq!(mid[8].kind, ProfessionScanKind::Farm);
    assert_eq!(mid[8].farm_job, Some(FarmProfession::WaterBringer));
    assert_eq!(mid[8].rung_label, "FILL_BUCKET");
    assert!(!mid[8].is_assigned_job);

    let hungry = plan_profession_ladder_steps(PriorityRung::ConsiderMakeFood, &sticky);
    assert_eq!(hungry.len(), 3);
    assert_eq!(hungry[0].kind, ProfessionScanKind::HandlingGraves);
    assert_eq!(hungry[1].kind, ProfessionScanKind::HandlingFire);
    assert_eq!(hungry[2].rung_label, crate::knife_stuff::KNIFE_STUFF_RUNG);
    assert_eq!(hungry[0].rung_label, "CONSIDER_MAKE_FOOD");

    let misc = plan_profession_ladder_steps(PriorityRung::CriticalMisc, &sticky);
    assert_eq!(misc[0].rung_label, crate::farmer_profession::PLACE_FLOOR_UNDER_RUNG);
    assert_eq!(misc[1].rung_label, crate::cleanup_profession::CRITICAL_CLEANUP_RUNG);
    assert_eq!(misc[2].rung_label, DO_WATERING_LOW_RUNG);
    assert_eq!(misc[3].rung_label, CRITICAL_STUFF_RUNG);
    assert_eq!(misc[4].kind, ProfessionScanKind::FireFood);
    assert_eq!(misc[5].kind, ProfessionScanKind::Baker);
    assert_eq!(misc[6].kind, ProfessionScanKind::Pottery);
    assert_eq!(misc[7].rung_label, DO_CARROT_LOW_RUNG);

    let rungs = npc_think_job_rungs(&sticky);
    assert_eq!(rungs[0], PriorityRung::CriticalCraft);
    assert_eq!(rungs[1], PriorityRung::MidPriorityTasks);
    assert_eq!(*rungs.last().unwrap(), PriorityRung::LowPriorityWork);
    assert!(rungs.contains(&PriorityRung::CriticalMisc));
    let assigned_sticky = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::BasicFarmer),
        age: 20.0,
        ..Default::default()
    };
    let rungs_a = npc_think_job_rungs(&assigned_sticky);
    assert!(rungs_a.contains(&PriorityRung::AssignedJob));

    let assigned_fk = ProfessionStickySnapshot {
        fire_keeper_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let steps = plan_assigned_job_steps(&assigned_fk);
    assert!(
        steps.iter().any(|s| s.kind == ProfessionScanKind::HandlingFire),
        "FIREKEEPER assigned must plan HandlingFire"
    );
    assert_eq!(
        crate::handling_fire_max_for_dispatch(true, "ASSIGNED_JOB"),
        crate::HANDLING_FIRE_ASSIGNED_MAX
    );

    let assigned_hunter = ProfessionStickySnapshot {
        hunter_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let hsteps = plan_assigned_job_steps(&assigned_hunter);
    assert!(
        hsteps.iter().any(|s| s.kind == ProfessionScanKind::Hunting),
        "HUNTER assigned must plan Hunting"
    );
    assert_eq!(
        crate::hunting_max_for_dispatch(true, hsteps[0].rung_label),
        crate::HUNTING_ASSIGNED_MAX
    );

    let assigned_lj = ProfessionStickySnapshot {
        lumberjack_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let lsteps = plan_assigned_job_steps(&assigned_lj);
    assert!(
        lsteps.iter().any(|s| s.kind == ProfessionScanKind::CuttingWood),
        "LUMBERJACK assigned must plan CuttingWood"
    );
    assert_eq!(
        crate::cutting_wood_max_for_dispatch(true, lsteps[0].rung_label),
        crate::CUTTING_WOOD_ASSIGNED_MAX
    );

    let assigned_col = ProfessionStickySnapshot {
        collector_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let csteps = plan_assigned_job_steps(&assigned_col);
    assert!(
        csteps.iter().any(|s| s.kind == ProfessionScanKind::Collecting),
        "COLLECTOR assigned must plan Collecting"
    );
    assert_eq!(
        crate::collecting_max_for_dispatch(true, csteps[0].rung_label),
        crate::COLLECTING_ASSIGNED_MAX
    );

    let assigned_wb = ProfessionStickySnapshot {
        farm_assigned: Some(FarmProfession::WaterBringer),
        age: 20.0,
        ..Default::default()
    };
    let wsteps = plan_assigned_job_steps(&assigned_wb);
    assert!(
        wsteps.iter().any(|s| s.kind == ProfessionScanKind::Farm
            && s.farm_job == Some(FarmProfession::WaterBringer)),
        "WATERBRINGER assigned must plan Farm WaterBringer"
    );
    assert_eq!(wsteps[0].rung_label, "ASSIGNED_JOB");
    assert_eq!(WATER_BRINGER_ASSIGNED_MAX_PEOPLE, 100);

    let assigned_fs = ProfessionStickySnapshot {
        foodserver_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let fsteps = plan_assigned_job_steps(&assigned_fs);
    assert!(
        fsteps.iter().any(|s| s.kind == ProfessionScanKind::FoodServer),
        "FOODSERVER assigned must plan FoodServer"
    );
    assert_eq!(
        crate::foodserver_max_for_dispatch(true, fsteps[0].rung_label),
        crate::FOODSERVER_ASSIGNED_MAX
    );

    let feed_mid = plan_profession_ladder_steps(
        PriorityRung::FeedPlayerInNeed,
        &ProfessionStickySnapshot {
            age: 20.0,
            ..Default::default()
        },
    );
    assert_eq!(feed_mid.len(), 1);
    assert_eq!(feed_mid[0].kind, ProfessionScanKind::FoodServer);
    assert_eq!(feed_mid[0].rung_label, "FEED_PLAYER_IN_NEED");
    assert!(!feed_mid[0].is_assigned_job);
    assert_eq!(
        crate::foodserver_max_for_dispatch(false, feed_mid[0].rung_label),
        crate::FOODSERVER_DEFAULT_MAX
    );

    let assigned_tailor = ProfessionStickySnapshot {
        tailor_assigned: true,
        age: 8.0,
        ..Default::default()
    };
    let tsteps = plan_assigned_job_steps(&assigned_tailor);
    assert!(
        tsteps.iter().any(|s| s.kind == ProfessionScanKind::Tailor),
        "TAILOR assigned must plan Tailor"
    );
    assert_eq!(
        crate::tailor_max_for_dispatch(true, tsteps[0].rung_label),
        crate::TAILOR_ASSIGNED_MAX
    );
    let last_tailor = ProfessionStickySnapshot {
        tailor_last: true,
        age: 20.0,
        ..Default::default()
    };
    let ltailor = plan_assigned_job_steps(&last_tailor);
    assert!(
        ltailor.iter().any(|s| s.kind == ProfessionScanKind::Tailor),
        "last TAILOR must plan Tailor"
    );

    let low = plan_profession_ladder_steps(PriorityRung::LowPriorityWork, &sticky);
    assert!(
        low.iter().any(|s| s.kind == ProfessionScanKind::CuttingWood),
        "LowPriorityWork must plan isCuttingWood"
    );
    assert_eq!(low[0].kind, ProfessionScanKind::Collecting);
    assert_eq!(low[1].kind, ProfessionScanKind::Farm);
    assert_eq!(low[1].farm_job, Some(FarmProfession::WaterBringer));
    assert_eq!(low[1].rung_label, DO_WATERING_LOW_RUNG);
    assert!(!low[1].is_assigned_job);
    assert_eq!(
        watering_max_for_dispatch(false, low[1].rung_label),
        WATER_BRINGER_LOW_MAX_PEOPLE
    );
    assert_eq!(low[2].kind, ProfessionScanKind::Farm);
    assert_eq!(low[2].farm_job, Some(FarmProfession::CarrotFarmer));
    assert_eq!(low[2].rung_label, DO_CARROT_LOW_RUNG);
    assert!(!low[2].is_assigned_job);
    assert_eq!(
        carrot_max_for_dispatch(false, low[2].rung_label),
        CARROT_FARMER_LOW_MAX_PEOPLE
    );
    assert_eq!(low[3].kind, ProfessionScanKind::Farm);
    assert_eq!(low[3].farm_job, None);
    assert_eq!(low[3].rung_label, FILL_BEAN_BOWL_RUNG);
    assert!(!low[3].is_assigned_job);

    let age_low = plan_profession_ladder_steps(PriorityRung::AgeRotatedJob, &sticky);
    assert_eq!(age_low[0].kind, ProfessionScanKind::Farm);
    assert_eq!(age_low[0].farm_job, Some(FarmProfession::WaterBringer));
    assert_eq!(age_low[0].rung_label, DO_WATERING_LOW_RUNG);
    assert_eq!(age_low[1].kind, ProfessionScanKind::Farm);
    assert_eq!(age_low[1].farm_job, Some(FarmProfession::CarrotFarmer));
    assert_eq!(age_low[1].rung_label, DO_CARROT_LOW_RUNG);
    assert_eq!(age_low[2].kind, ProfessionScanKind::Farm);
    assert_eq!(age_low[2].farm_job, None);
    assert_eq!(age_low[2].rung_label, FILL_BEAN_BOWL_RUNG);

    let assigned_gk = ProfessionStickySnapshot {
        grave_keeper_assigned: true,
        age: 20.0,
        ..Default::default()
    };
    let gsteps = plan_assigned_job_steps(&assigned_gk);
    assert!(
        gsteps
            .iter()
            .any(|s| s.kind == ProfessionScanKind::HandlingGraves),
        "GRAVEKEEPER assigned must plan HandlingGraves"
    );
}

#[test]
fn handling_graves_scan_removes_cargo() {
    use crate::handling_graves::{GRAVE_88, HandlingGravesAction};
    let tiles = vec![
        ScanTile {
            parent_id: GRAVE_88,
            x: 2,
            y: 0,
            contained_count: 1,
            ..ScanTile::simple(GRAVE_88, 2, 0)
        },
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_best_grave_keeper = true;
    let mut gk = crate::GraveKeeperProfessionRuntime {
        is_last_grave_keeper: true,
        weight: 1.0,
        ..Default::default()
    };
    let r = handling_graves_profession_scan_tick(&tiles, &inp, "MID_PRIORITY_TASKS", &mut gk);
    assert!(r.had_action, "cargo grave should act {:?}", r.intent);
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::StageRemoveFromContainer {
                x: 2,
                y: 0,
                expected_parent: GRAVE_88
            }
        ),
        "unexpected {:?}",
        r.intent
    );
    let _ = HandlingGravesAction::None;
}

#[test]
fn apply_profession_scan_tick_stages_remove_from_container() {
    use std::sync::Arc;
    use crate::handling_graves::GRAVE_88;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, GRAVE_88);
        assert!(w.container_put(2, 0, 33, 4));
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.home_x = 0;
    p.home_y = 0;
    p.x = 0;
    p.y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.held_id = 0;
    p.grave_keeper_profession.is_last_grave_keeper = true;
    p.grave_keeper_profession.weight = 1.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingGraves,
        "MID_PRIORITY_TASKS",
    );
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)),
        "first tick only stages {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    let st = p.craft_ai.remove_from_container.expect("sticky");
    assert_eq!((st.tx, st.ty, st.expected_parent), (2, 0, GRAVE_88));
}

#[test]
fn apply_remove_from_container_tick_remvs_when_adjacent() {
    use std::sync::Arc;
    use crate::handling_graves::GRAVE_88;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(1, 0, GRAVE_88);
        assert!(w.container_put(1, 0, 77, 4));
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.x = 0;
    p.y = 0;
    p.held_id = 0;
    p.craft_ai.remove_from_container = crate::stage_remove_item_from_container(1, 0, GRAVE_88, true, false);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = crate::apply_player_remove_from_container_tick(&mut state, &hub, 1)
        .expect("sticky tick");
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Dropped),
        "adjacent remv {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(p.held_id, 77);
    assert!(p.craft_ai.remove_from_container.is_none());
}

#[test]
fn apply_profession_scan_tick_grave_keeper_drops_bones_basket() {
    use std::sync::Arc;
    use crate::handling_graves::{BASKET_OF_BONES, GRAVE_88};
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, GRAVE_88);
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.home_x = 0;
    p.home_y = 0;
    p.x = 0;
    p.y = 0;
    p.age = 20.0;
    p.food = 5.0;
    p.held_id = BASKET_OF_BONES;
    p.grave_keeper_profession.is_last_grave_keeper = true;
    p.grave_keeper_profession.weight = 1.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingGraves,
        "MID_PRIORITY_TASKS",
    );
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Dropped | ShortCraftLiveApplyResult::Staging(_)),
        "drop basket of bones {:?}",
        r
    );
}

#[test]
fn is_self_best_grave_keeper_from_state_closer_weight_wins() {
    // Haxe: getBestAiForObjByProfession GRAVEKEEPER vs grave; weight>0 + closer wins
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut far = crate::Player::new(1, 1, "far@t");
    far.home_x = 10;
    far.home_y = 10;
    far.x = 20;
    far.y = 10;
    far.age = 20.0;
    far.food = 5.0;
    let mut close = crate::Player::new(2, 2, "close@t");
    close.home_x = 10;
    close.home_y = 10;
    close.x = 11;
    close.y = 10;
    close.age = 20.0;
    close.food = 5.0;
    close.grave_keeper_profession.weight = 1.0;
    state.players.insert(1, far);
    state.players.insert(2, close);
    assert!(
        !is_self_best_grave_keeper_from_state(&state, 1, 10, 10, 10, 10),
        "far self without weight loses to closer GRAVEKEEPER"
    );
    assert!(
        is_self_best_grave_keeper_from_state(&state, 2, 10, 10, 10, 10),
        "closer weight wins"
    );
}

#[test]
fn apply_profession_scan_tick_far_peer_not_best_skips_new_grave() {
    use std::sync::Arc;
    use crate::handling_graves::GRAVE_88;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(10, 10, GRAVE_88);
    }
    let mut far = crate::Player::new(1, 1, "far@t");
    far.home_x = 10;
    far.home_y = 10;
    far.x = 20;
    far.y = 10;
    far.age = 20.0;
    far.food = 5.0;
    let mut close = crate::Player::new(2, 2, "close@t");
    close.home_x = 10;
    close.home_y = 10;
    close.x = 11;
    close.y = 10;
    close.age = 20.0;
    close.food = 5.0;
    close.grave_keeper_profession.weight = 1.0;
    state.players.insert(1, far);
    state.players.insert(2, close);
    let hub = ol_net::OutboundHub::new();
    let _ = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingGraves,
        "MID_PRIORITY_TASKS",
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.grave_keeper_profession.weight, 0.0,
        "getBestAi zeros self when another AI wins"
    );
}

#[test]
fn apply_handle_death_tick_wipes_jobs_and_drops_at_home() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "old@t");
    p.home_x = 0;
    p.home_y = 0;
    p.x = 0;
    p.y = 0;
    p.age = 58.0;
    p.food = 10.0;
    p.held_id = 33;
    p.smith_profession.is_last_smith = true;
    p.smith_profession.stage = 1.0;
    p.last_profession = Some("SMITH".into());
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_death_tick(&mut state, &hub, 1);
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Dropped | ShortCraftLiveApplyResult::Staging(_)
        ),
        "near-home no grave should drop or wait {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.grave_keeper_profession.is_last_grave_keeper);
    assert_eq!(p.grave_keeper_profession.weight, 1.0);
    assert!(!p.smith_profession.is_last_smith);
    assert_eq!(p.last_profession.as_deref(), Some("GRAVEKEEPER"));
}

#[test]
fn apply_handle_death_tick_young_skips() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "young@t");
    p.age = 20.0;
    p.smith_profession.is_last_smith = true;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_death_tick(&mut state, &hub, 1);
    assert!(matches!(r, ShortCraftLiveApplyResult::Failed));
    let p = state.players.get(&1).unwrap();
    assert!(p.smith_profession.is_last_smith);
}

#[test]
fn apply_handle_death_tick_far_goes_home() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "far@t");
    p.home_x = 10;
    p.home_y = 10;
    p.x = 40;
    p.y = 10;
    p.age = 58.0;
    p.food = 10.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_death_tick(&mut state, &hub, 1);
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { .. })),
        "far from home should goto {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.grave_keeper_profession.is_last_grave_keeper);
}

#[test]
fn apply_handle_temperature_tick_drinks_held_water() {
    use std::sync::Arc;
    use crate::clothing_transitions::WATER_BOWL_ID;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "hot@t");
    p.x = 0;
    p.y = 0;
    p.heat = 0.9;
    p.held_id = WATER_BOWL_ID;
    p.age = 20.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)),
        "drink should consume tick {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_ne!(p.held_id, WATER_BOWL_ID, "bowl should empty after drink");
    assert!(p.heat < 0.9);
}

#[test]
fn apply_handle_temperature_tick_gotos_snow_when_sticky_hot() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_biome(4, 0, 4);
    }
    let mut p = crate::Player::new(1, 1, "cool@t");
    p.x = 0;
    p.y = 0;
    p.heat = 0.65;
    p.ai_handling_temperature = true;
    p.age = 20.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 4, y: 0 })
        ),
        "sticky cooling should goto snow {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.ai_handling_temperature);
}

#[test]
fn apply_handle_temperature_tick_idle_when_comfortable() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "ok@t");
    p.heat = 0.5;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(matches!(r, ShortCraftLiveApplyResult::Failed));
    let p = state.players.get(&1).unwrap();
    assert!(!p.ai_handling_temperature);
}

#[test]
fn apply_handle_temperature_tick_first_arrival_relaxes() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "arrive@t");
    p.x = 0;
    p.y = 0;
    p.heat = 0.65;
    p.ai_handling_temperature = true;
    p.ai_temp_just_arrived = false;
    p.cold_place = Some((0, 0));
    p.age = 20.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)),
        "first arrival time+=3 relax {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.ai_temp_just_arrived);
    assert!(p.ai_handling_temperature);
}

#[test]
fn apply_handle_temperature_tick_fail_cool_clears_cold_place() {
    // Haxe L1715–1721: arrived, heat still high → coldPlace = null; return false
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "failcool@t");
    p.x = 0;
    p.y = 0;
    p.heat = 0.7;
    p.ai_last_heat = 0.6;
    p.last_temperature = 0.5;
    p.ai_handling_temperature = true;
    p.ai_temp_just_arrived = true;
    p.cold_place = Some((0, 0));
    p.age = 20.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(matches!(r, ShortCraftLiveApplyResult::Failed));
    let p = state.players.get(&1).unwrap();
    assert!(p.cold_place.is_none());
    assert!(p.ai_handling_temperature);
}

#[test]
fn apply_handle_temperature_tick_fail_warm_young_clears_warm_place() {
    // Haxe L1725–1747: fail-warm age≤5 skips kindling/isHandlingFire; warmPlace = null
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "failwarm@t");
    p.x = 0;
    p.y = 0;
    p.heat = 0.2;
    p.ai_last_heat = 0.3;
    p.last_temperature = 0.4;
    p.ai_handling_temperature = true;
    p.ai_temp_just_arrived = true;
    p.warm_place = Some((0, 0));
    p.age = 3.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_handle_temperature_tick(&mut state, &hub, 1);
    assert!(matches!(r, ShortCraftLiveApplyResult::Failed));
    let p = state.players.get(&1).unwrap();
    assert!(p.warm_place.is_none());
}

#[test]
fn apply_profession_scan_tick_hunts_snake_near_home() {
    use std::sync::Arc;
    use crate::{HUNT_KNIFE, RATTLE_SNAKE};
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(5, 0, RATTLE_SNAKE);
    }
    let mut p = crate::Player::new(1, 1, "hunt@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.held_id = HUNT_KNIFE;
    p.hunter_profession.is_last_hunter = true;
    p.hunter_profession.is_assigned_hunter = true;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Hunting,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt { .. })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { .. })
        ),
        "assigned hunter with knife+snake should USE/goto {:?}",
        r
    );
}

#[test]
fn apply_profession_scan_tick_cuts_firewood_near_fire() {
    use std::sync::Arc;
    use crate::FIREWOOD;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "lumber@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.ai_fire_place_id = crate::FIRE;
    p.ai_fire_place_x = 0;
    p.ai_fire_place_y = 0;
    p.lumberjack_profession.is_last_lumberjack = true;
    p.lumberjack_profession.is_assigned_lumberjack = true;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::CuttingWood,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::CraftItem {
                object_id: FIREWOOD
            })
        ),
        "assigned lumberjack with firePlace and no wood should craft firewood {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.lumberjack_profession.is_last_lumberjack);
    assert_eq!(p.lumberjack_profession.stage, 1.0);
}

#[test]
fn apply_profession_scan_tick_collects_kindling_near_home() {
    use std::sync::Arc;
    use crate::COLLECT_KINDLING;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "collect@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.collector_profession.is_last_collector = true;
    p.collector_profession.is_assigned_collector = true;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Collecting,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::CraftItem {
                object_id: COLLECT_KINDLING
            })
        ),
        "assigned collector with no kindling should craft kindling {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.collector_profession.is_last_collector);
    assert_eq!(p.collector_profession.task_kindling, 1.0);
}

#[test]
fn farm_profession_scan_tick_assigned_waterbringer_waters_closest_dry() {
    let tiles = vec![
        ScanTile::simple(DRY_PLANTED_CARROTS, 10, 0),
        ScanTile::simple(DRY_PLANTED_WHEAT, 1, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.farm_peer_lasts = Some(vec![
        FarmProfession::WaterBringer,
        FarmProfession::WaterBringer,
        FarmProfession::WaterBringer,
    ]);
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime {
        assigned_profession: Some(FarmProfession::WaterBringer),
        last_profession: Some(FarmProfession::WaterBringer),
        ..Default::default()
    };
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::WaterBringer),
        "ASSIGNED_JOB",
        &mut task,
        true,
        &mut rt,
    );
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: WET_PLANTED_WHEAT
        },
        "closest wheat must beat list-order carrots; assigned max 100 vs 3 peers"
    );
    assert_eq!(rt.last_profession, Some(FarmProfession::WaterBringer));
}

#[test]
fn farm_profession_scan_tick_assigned_waterbringer_waters_dry_carrots() {
    let tiles = vec![ScanTile::simple(DRY_PLANTED_CARROTS, 2, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime {
        assigned_profession: Some(FarmProfession::WaterBringer),
        ..Default::default()
    };
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::WaterBringer),
        "ASSIGNED_JOB",
        &mut task,
        true,
        &mut rt,
    );
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: WET_PLANTED_CARROTS
        }
    );
    assert_eq!(rt.last_profession, Some(FarmProfession::WaterBringer));
}

#[test]
fn farm_profession_scan_tick_low_watering_waters_closest_and_peer_caps() {
    // Haxe doWatering(1): hasOrBecomeProfession WATERBRINGER max=1 then closest helper.
    let tiles = vec![
        ScanTile::simple(DRY_PLANTED_CARROTS, 10, 0),
        ScanTile::simple(DRY_PLANTED_WHEAT, 1, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::WaterBringer),
        DO_WATERING_LOW_RUNG,
        &mut task,
        true,
        &mut rt,
    );
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::CraftItem {
            object_id: WET_PLANTED_WHEAT
        },
        "low doWatering(1) must water closest wheat not list-order carrots"
    );
    assert_eq!(rt.last_profession, Some(FarmProfession::WaterBringer));

    let mut inp_cap = ProfessionScanInput::basic(0, 0, 0);
    inp_cap.is_assigned_job = false;
    inp_cap.farm_peer_lasts = Some(vec![FarmProfession::WaterBringer]);
    let mut task2 = FarmTaskState::default();
    let mut rt2 = FarmProfessionRuntime::default();
    let r2 = farm_profession_scan_tick(
        &tiles,
        &inp_cap,
        Some(FarmProfession::WaterBringer),
        DO_WATERING_LOW_RUNG,
        &mut task2,
        true,
        &mut rt2,
    );
    assert!(
        !r2.had_action,
        "one WATERBRINGER peer must refuse doWatering(1) {:?}",
        r2.intent
    );
    assert_ne!(rt2.last_profession, Some(FarmProfession::WaterBringer));
}

#[test]
fn farm_profession_scan_tick_low_carrot_pulls_row_and_peer_caps() {
    // Haxe doCarrotFarming(1): hasOrBecomeProfession CARROTFARMER max=1 then pull 400.
    let tiles = vec![ScanTile::simple(CARROT_ROW, 2, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::CarrotFarmer),
        DO_CARROT_LOW_RUNG,
        &mut task,
        true,
        &mut rt,
    );
    assert!(r.had_action, "low doCarrotFarming(1) should pull carrot row");
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: CARROT_ROW,
                actor_id: 0
            }
        ),
        "empty-hand shortCraft(0,400) {:?}",
        r.intent
    );
    assert_eq!(rt.last_profession, Some(FarmProfession::CarrotFarmer));

    let mut inp_cap = ProfessionScanInput::basic(0, 0, 0);
    inp_cap.is_assigned_job = false;
    inp_cap.farm_peer_lasts = Some(vec![FarmProfession::CarrotFarmer]);
    let mut task2 = FarmTaskState::default();
    let mut rt2 = FarmProfessionRuntime::default();
    let r2 = farm_profession_scan_tick(
        &tiles,
        &inp_cap,
        Some(FarmProfession::CarrotFarmer),
        DO_CARROT_LOW_RUNG,
        &mut task2,
        true,
        &mut rt2,
    );
    assert!(
        !r2.had_action,
        "one CARROTFARMER peer must refuse doCarrotFarming(1) {:?}",
        r2.intent
    );
    assert_ne!(rt2.last_profession, Some(FarmProfession::CarrotFarmer));
}

#[test]
fn apply_profession_scan_tick_waters_dry_carrots_assigned_waterbringer() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, DRY_PLANTED_CARROTS);
    }
    let mut p = crate::Player::new(1, 1, "water@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.farm_profession.assigned_profession = Some(FarmProfession::WaterBringer);
    p.farm_profession.last_profession = Some(FarmProfession::WaterBringer);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::CraftItem {
                object_id: WET_PLANTED_CARROTS
            })
        ),
        "assigned WATERBRINGER with dry carrots should craft wet carrots {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.farm_profession.last_profession,
        Some(FarmProfession::WaterBringer)
    );
}

#[test]
fn apply_profession_scan_tick_waters_dry_carrots_low_watering() {
    // Haxe doWatering(1): unassigned AI becomes WATERBRINGER when peer room.
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, DRY_PLANTED_CARROTS);
    }
    let mut p = crate::Player::new(1, 1, "water-low@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        DO_WATERING_LOW_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::CraftItem {
                object_id: WET_PLANTED_CARROTS
            })
        ),
        "low doWatering(1) with dry carrots should craft wet carrots {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.farm_profession.last_profession,
        Some(FarmProfession::WaterBringer)
    );
}

#[test]
fn apply_profession_scan_tick_pulls_carrot_row_low_carrot() {
    // Haxe doCarrotFarming(1): unassigned AI becomes CARROTFARMER when peer room.
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, CARROT_ROW);
        // Haxe hasCarrotSeeds: countSeeds(401+2745) > 1 else shortCraft(0,400) refuses uses<4
        w.set_object(1, 1, SEEDING_CARROTS);
        w.set_object(2, 2, BOWL_OF_CARROT_SEEDS);
    }
    let mut p = crate::Player::new(1, 1, "carrot-low@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        DO_CARROT_LOW_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: CARROT_ROW,
                    actor_id: 0
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "low doCarrotFarming(1) with carrot row should USE/goto {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.farm_profession.last_profession,
        Some(FarmProfession::CarrotFarmer)
    );
}

#[test]
fn farm_profession_scan_tick_fill_bean_bowl_uses_held_on_plant() {
    // Haxe fillBeanBowlIfNeeded(): held 1175 + plant 1173 + dry stock [1176,1172].
    let tiles = vec![
        ScanTile::simple(GREEN_BEAN_PLANTS, 2, 0),
        ScanTile::simple(DRY_BEAN_PLANTS, 1, 1),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, BOWL_OF_GREEN_BEANS);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        FILL_BEAN_BOWL_RUNG,
        &mut task,
        false,
        &mut rt,
    );
    assert!(r.had_action, "low fillBeanBowlIfNeeded should USE plant");
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: GREEN_BEAN_PLANTS,
                actor_id: BOWL_OF_GREEN_BEANS
            }
        ),
        "held green bowl on plant {:?}",
        r.intent
    );
    assert_eq!(rt.last_profession, None);

    let mut inp_cap = ProfessionScanInput::basic(0, 0, 0);
    inp_cap.is_assigned_job = false;
    inp_cap.is_best_bowl_filler = false;
    let mut task2 = FarmTaskState::default();
    let mut rt2 = FarmProfessionRuntime::default();
    let r2 = farm_profession_scan_tick(
        &tiles,
        &inp_cap,
        None,
        FILL_BEAN_BOWL_RUNG,
        &mut task2,
        false,
        &mut rt2,
    );
    assert!(
        !r2.had_action,
        "not BowlFiller must skip pickup/GetItem {:?}",
        r2.intent
    );
}

#[test]
fn apply_profession_scan_tick_fill_bean_bowl_uses_held_green() {
    // Haxe fillBeanBowlIfNeeded(): unassigned AI fills held green bowl on plant.
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, GREEN_BEAN_PLANTS);
        w.set_object(1, 1, DRY_BEAN_PLANTS);
    }
    let mut p = crate::Player::new(1, 1, "bean-bowl@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.set_held(BOWL_OF_GREEN_BEANS, 1);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        FILL_BEAN_BOWL_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: GREEN_BEAN_PLANTS,
                    actor_id: BOWL_OF_GREEN_BEANS
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "low fillBeanBowlIfNeeded with held bowl should USE/goto plant {:?}",
        r
    );
}

#[test]
fn farm_profession_scan_tick_fill_bean_held_green_then_dry_skips_pickup() {
    // Haxe fillBeanBowlIfNeeded(*, true): held USE only; empty hands skip GetItem.
    let tiles = vec![
        ScanTile::simple(GREEN_BEAN_PLANTS, 2, 0),
        ScanTile::simple(DRY_BEAN_PLANTS, 3, 0),
        ScanTile::simple(BOWL_OF_DRY_BEANS, 1, 1),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, BOWL_OF_GREEN_BEANS);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        FILL_BEAN_HELD_RUNG,
        &mut task,
        false,
        &mut rt,
    );
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: GREEN_BEAN_PLANTS,
                actor_id: BOWL_OF_GREEN_BEANS
            }
        ),
        "mid onlyFillHeld green should USE plant {:?}",
        r.intent
    );

    let mut inp_dry = ProfessionScanInput::basic(0, 0, BOWL_OF_DRY_BEANS);
    inp_dry.is_assigned_job = false;
    let mut task_d = FarmTaskState::default();
    let mut rt_d = FarmProfessionRuntime::default();
    let rd = farm_profession_scan_tick(
        &tiles,
        &inp_dry,
        None,
        FILL_BEAN_HELD_RUNG,
        &mut task_d,
        false,
        &mut rt_d,
    );
    assert!(
        matches!(
            rd.intent,
            ShortCraftLiveIntent::UseAt {
                x: 3,
                y: 0,
                target_id: DRY_BEAN_PLANTS,
                actor_id: BOWL_OF_DRY_BEANS
            }
        ),
        "mid onlyFillHeld dry should USE dry plant {:?}",
        rd.intent
    );

    let mut inp_empty = ProfessionScanInput::basic(0, 0, 0);
    inp_empty.is_assigned_job = false;
    let mut task_e = FarmTaskState::default();
    let mut rt_e = FarmProfessionRuntime::default();
    let re = farm_profession_scan_tick(
        &tiles,
        &inp_empty,
        None,
        FILL_BEAN_HELD_RUNG,
        &mut task_e,
        false,
        &mut rt_e,
    );
    assert!(
        !re.had_action,
        "onlyFillHeld empty hands must skip pickup/GetItem {:?}",
        re.intent
    );
}

#[test]
fn apply_profession_scan_tick_fill_bean_held_uses_held_green() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, GREEN_BEAN_PLANTS);
        w.set_object(1, 1, DRY_BEAN_PLANTS);
    }
    let mut p = crate::Player::new(1, 1, "bean-held@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.set_held(BOWL_OF_GREEN_BEANS, 1);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        FILL_BEAN_HELD_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: GREEN_BEAN_PLANTS,
                    actor_id: BOWL_OF_GREEN_BEANS
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "mid onlyFillHeld with held green bowl should USE/goto plant {:?}",
        r
    );
}

#[test]
fn farm_profession_scan_tick_pull_carrot_row_uses_empty_hand_r10() {
    // Haxe shortCraft(0, 400, 10): empty-hand USE within r=10; far/no-seed skip.
    let tiles = vec![ScanTile::simple(CARROT_ROW, 2, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        PULL_CARROT_ROW_RUNG,
        &mut task,
        false,
        &mut rt,
    );
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: CARROT_ROW,
                actor_id: 0
            }
        ),
        "mid shortCraft(0,400,10) should USE carrot row {:?}",
        r.intent
    );
    assert_eq!(rt.last_profession, None);

    let far = vec![ScanTile::simple(CARROT_ROW, 11, 0)];
    let mut task_f = FarmTaskState::default();
    let mut rt_f = FarmProfessionRuntime::default();
    let rf = farm_profession_scan_tick(
        &far,
        &inp,
        None,
        PULL_CARROT_ROW_RUNG,
        &mut task_f,
        false,
        &mut rt_f,
    );
    assert!(
        !rf.had_action,
        "row at Chebyshev 11 must skip r=10 {:?}",
        rf.intent
    );

    let mut inp_seed = ProfessionScanInput::basic(0, 0, 0);
    inp_seed.is_assigned_job = false;
    inp_seed.has_carrot_seeds = false;
    let mut task_s = FarmTaskState::default();
    let mut rt_s = FarmProfessionRuntime::default();
    let rs = farm_profession_scan_tick(
        &tiles,
        &inp_seed,
        None,
        PULL_CARROT_ROW_RUNG,
        &mut task_s,
        false,
        &mut rt_s,
    );
    assert!(
        !rs.had_action,
        "no seeds + uses<4 must refuse {:?}",
        rs.intent
    );

    let ripe = vec![ScanTile::simple(CARROT_ROW, 2, 0).with_uses(4)];
    let mut task_r = FarmTaskState::default();
    let mut rt_r = FarmProfessionRuntime::default();
    let rr = farm_profession_scan_tick(
        &ripe,
        &inp_seed,
        None,
        PULL_CARROT_ROW_RUNG,
        &mut task_r,
        false,
        &mut rt_r,
    );
    assert!(
        matches!(
            rr.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: CARROT_ROW,
                actor_id: 0
            }
        ),
        "uses>=4 without seeds should still pull {:?}",
        rr.intent
    );

    let mut inp_hold = ProfessionScanInput::basic(0, 0, BOWL_OF_GREEN_BEANS);
    inp_hold.is_assigned_job = false;
    let mut task_h = FarmTaskState::default();
    let mut rt_h = FarmProfessionRuntime::default();
    let rh = farm_profession_scan_tick(
        &tiles,
        &inp_hold,
        None,
        PULL_CARROT_ROW_RUNG,
        &mut task_h,
        false,
        &mut rt_h,
    );
    assert!(rh.had_action, "held actor0 should drop {:?}", rh.intent);
    assert!(
        !matches!(
            rh.intent,
            ShortCraftLiveIntent::UseAt {
                target_id: CARROT_ROW,
                actor_id: 0,
                ..
            }
        ),
        "held shortCraft(0,400) must drop not USE {:?}",
        rh.intent
    );
}

#[test]
fn apply_profession_scan_tick_pull_carrot_row_uses_empty_hand() {
    // Haxe mid shortCraft(0,400,10): no CARROTFARMER profession.
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, CARROT_ROW);
        w.set_object(1, 1, SEEDING_CARROTS);
        w.set_object(2, 2, BOWL_OF_CARROT_SEEDS);
    }
    let mut p = crate::Player::new(1, 1, "pull-carrot@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        PULL_CARROT_ROW_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: CARROT_ROW,
                    actor_id: 0
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "mid shortCraft(0,400,10) should USE/goto carrot row {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(p.farm_profession.last_profession, None);
}

#[test]
fn farm_profession_scan_tick_fill_berry_held_uses_bush() {
    // Haxe fillBerryBowlIfNeeded(true): held 253 + closest bush r=20.
    let tiles = vec![ScanTile::simple(WILD_BUSH, 2, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, BOWL_GOOSEBERRIES);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        FILL_BERRY_HELD_RUNG,
        &mut task,
        false,
        &mut rt,
    );
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 2,
                y: 0,
                target_id: WILD_BUSH,
                actor_id: BOWL_GOOSEBERRIES
            }
        ),
        "mid fillBerryBowlIfNeeded(true) should USE bush {:?}",
        r.intent
    );
    let mut inp_empty = ProfessionScanInput::basic(0, 0, 0);
    inp_empty.is_assigned_job = false;
    let mut task_e = FarmTaskState::default();
    let mut rt_e = FarmProfessionRuntime::default();
    let re = farm_profession_scan_tick(
        &tiles,
        &inp_empty,
        None,
        FILL_BERRY_HELD_RUNG,
        &mut task_e,
        false,
        &mut rt_e,
    );
    assert!(
        !re.had_action,
        "onlyFillHeld empty hands must skip {:?}",
        re.intent
    );
}

#[test]
fn apply_profession_scan_tick_fill_berry_held_uses_held_bowl() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, WILD_BUSH);
    }
    let mut p = crate::Player::new(1, 1, "berry-held@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.set_held(BOWL_GOOSEBERRIES, 1);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        FILL_BERRY_HELD_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: WILD_BUSH,
                    actor_id: BOWL_GOOSEBERRIES
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "mid fillBerryBowlIfNeeded(true) should USE/goto bush {:?}",
        r
    );
}

#[test]
fn farm_profession_scan_tick_knife_stuff_uses_bread_not_bear() {
    use crate::knife_stuff::{
        BAKED_BREAD, DEAD_GRIZZLY_BEAR, DEAD_WOLF, KNIFE, KNIFE_STUFF_RUNG, LEAVENED_DOUGH_PLATE,
    };
    let tiles = vec![
        ScanTile::simple(DEAD_GRIZZLY_BEAR, 1, 0),
        ScanTile::simple(DEAD_WOLF, 2, 0),
        ScanTile::simple(LEAVENED_DOUGH_PLATE, 3, 0),
        ScanTile::simple(BAKED_BREAD, 4, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, KNIFE);
    inp.is_assigned_job = false;
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        KNIFE_STUFF_RUNG,
        &mut task,
        false,
        &mut rt,
    );
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt {
                x: 4,
                y: 0,
                target_id: BAKED_BREAD,
                actor_id: KNIFE
            }
        ),
        "held knife + bread r=20 should USE bread first {:?}",
        r.intent
    );
    let mut inp_empty = ProfessionScanInput::basic(0, 0, 0);
    inp_empty.is_assigned_job = false;
    let mut task_e = FarmTaskState::default();
    let mut rt_e = FarmProfessionRuntime::default();
    let re = farm_profession_scan_tick(
        &tiles,
        &inp_empty,
        None,
        KNIFE_STUFF_RUNG,
        &mut task_e,
        false,
        &mut rt_e,
    );
    assert!(
        !re.had_action,
        "held not knife must not seek/craft 560 {:?}",
        re.intent
    );
    assert!(!matches!(
        re.intent,
        ShortCraftLiveIntent::CraftItem { object_id: KNIFE }
            | ShortCraftLiveIntent::SeekOrCraft { actor: KNIFE, .. }
    ));
}

#[test]
fn apply_profession_scan_tick_knife_stuff_uses_held_knife_on_bread() {
    use crate::knife_stuff::{BAKED_BREAD, KNIFE, KNIFE_STUFF_RUNG};
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(2, 0, BAKED_BREAD);
    }
    let mut p = crate::Player::new(1, 1, "knife-stuff@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.set_held(KNIFE, 0);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        KNIFE_STUFF_RUNG,
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Used(_)
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::UseAt {
                    x: 2,
                    y: 0,
                    target_id: BAKED_BREAD,
                    actor_id: KNIFE
                })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: 2, y: 0 })
        ),
        "mid doKnifeStuff should USE/goto baked bread {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(p.farm_profession.last_profession, None);
}

#[test]
fn apply_profession_scan_tick_fill_bucket_drops_held_full() {
    // Haxe fillBucketIfNeeded: held Full Bucket 660 → dropHeldObject(0)
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "bucket@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 20.0;
    p.food = 10.0;
    p.set_held(660, 0);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Farm,
        "FILL_BUCKET",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Dropped
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::DropAt { x: 0, y: 0 })
        ),
        "fillBucketIfNeeded holding full bucket should drop {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.farm_profession.last_profession,
        Some(FarmProfession::WaterBringer)
    );
}

#[test]
fn apply_profession_scan_tick_feeds_starving_assigned_foodserver() {
    use std::sync::Arc;
    use ol_content::{ContentDb, ObjectDef};
    let mut db = ContentDb::default();
    let mut berry = ObjectDef::empty(31);
    berry.food_value = 5;
    db.objects.insert(31, berry);
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    let mut feeder = crate::Player::new(1, 1, "feeder@t");
    feeder.x = 0;
    feeder.y = 0;
    feeder.home_x = 0;
    feeder.home_y = 0;
    feeder.age = 20.0;
    feeder.food = 10.0;
    feeder.food_max = 20.0;
    feeder.set_held(31, 0);
    feeder.foodserver_profession.is_assigned_foodserver = true;
    feeder.foodserver_profession.is_last_foodserver = true;
    feeder.foodserver_profession.weight = 1.0;
    state.players.insert(1, feeder);
    let mut eater = crate::Player::new(2, 2, "eater@t");
    eater.x = 1;
    eater.y = 0;
    eater.home_x = 0;
    eater.home_y = 0;
    eater.age = 20.0;
    eater.food = 0.0;
    eater.food_max = 20.0;
    state.players.insert(2, eater);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::FoodServer,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Dropped
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::FeedOther { .. })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { .. })
        ),
        "assigned FOODSERVER should feed or goto starving {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.foodserver_profession.is_last_foodserver);
}

#[test]
fn apply_profession_scan_tick_feeds_starving_mid_max1() {
    // Haxe mid isFeedingPlayerInNeed() max=1 (SMITH<1); not assigned 100
    use std::sync::Arc;
    use ol_content::{ContentDb, ObjectDef};
    let mut db = ContentDb::default();
    let mut berry = ObjectDef::empty(31);
    berry.food_value = 5;
    db.objects.insert(31, berry);
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    let mut feeder = crate::Player::new(1, 1, "midfeed@t");
    feeder.x = 0;
    feeder.y = 0;
    feeder.home_x = 0;
    feeder.home_y = 0;
    feeder.age = 20.0;
    feeder.food = 10.0;
    feeder.food_max = 20.0;
    feeder.set_held(31, 0);
    state.players.insert(1, feeder);
    let mut eater = crate::Player::new(2, 2, "eater@t");
    eater.x = 1;
    eater.y = 0;
    eater.home_x = 0;
    eater.home_y = 0;
    eater.age = 20.0;
    eater.food = 0.0;
    eater.food_max = 20.0;
    state.players.insert(2, eater);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::FoodServer,
        "FEED_PLAYER_IN_NEED",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Dropped
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::FeedOther { .. })
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { .. })
        ),
        "mid isFeedingPlayerInNeed max=1 should feed or goto starving {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert!(p.foodserver_profession.is_last_foodserver);

    // Peer cap: two last FOODSERVERs → count 2 >= max 1 + wasIdle 1
    let mut mk_last = |conn: u64, pid: i32, x: i32| {
        let mut o = crate::Player::new(pid, conn, "fs@t");
        o.x = x;
        o.y = 0;
        o.home_x = 0;
        o.home_y = 0;
        o.age = 20.0;
        o.food = 10.0;
        o.food_max = 20.0;
        o.foodserver_profession.is_last_foodserver = true;
        o.last_profession = Some(crate::FOODSERVER_PROFESSION_KEY.into());
        o
    };
    state.players.insert(3, mk_last(3, 3, 2));
    state.players.insert(4, mk_last(4, 4, 3));
    if let Some(p) = state.players.get_mut(&1) {
        p.foodserver_profession = crate::FoodServerProfessionRuntime::default();
        p.last_profession = None;
    }
    let r2 = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::FoodServer,
        "FEED_PLAYER_IN_NEED",
    );
    assert!(
        matches!(
            r2,
            ShortCraftLiveApplyResult::Failed
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::None)
        ),
        "mid max=1 should refuse when two last FOODSERVERs at home {:?}",
        r2
    );
    assert!(
        !state
            .players
            .get(&1)
            .unwrap()
            .foodserver_profession
            .is_last_foodserver,
        "peer cap must not assign last FOODSERVER"
    );
}

#[test]
fn apply_profession_scan_tick_crafts_clothing_assigned_tailor() {
    // Haxe assigned TAILOR craftMediumPriorityClothing(100) even if age <= 10
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut p = crate::Player::new(1, 1, "tailor@t");
    p.x = 0;
    p.y = 0;
    p.home_x = 0;
    p.home_y = 0;
    p.age = 8.0;
    p.food = 10.0;
    p.assigned_profession = Some(crate::TAILOR_PROFESSION_KEY.into());
    // Fill bottom so high reed-skirt skips; fill back so fillUpQuiver empty-quiver skips.
    // Assigned medium then wants Empty Water Pouch 209.
    p.set_clothing_index_helper(4, Some(ol_world::NestedHelper::id_only(128)));
    p.set_clothing_index_helper(5, Some(ol_world::NestedHelper::id_only(198)));
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::Tailor,
        "ASSIGNED_JOB",
    );
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::CraftItem { object_id: 209 })
        ),
        "assigned TAILOR age 8 should medium-craft water pouch {:?}",
        r
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.last_profession.as_deref(),
        Some(crate::TAILOR_PROFESSION_KEY)
    );
}

#[test]
fn late_make_fire_food_scan_tick_max1_peer_cap() {
    use crate::baker_profession::RAW_MUTTON;
    use crate::HOT_COALS;
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 1, 0),
        ScanTile::simple(RAW_MUTTON, 0, 1),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.peer_count = 0.0;
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let r = late_make_fire_food_scan_tick(&tiles, &inp, &mut fire_rt);
    assert!(r.had_action, "late makeFireFood(1) should act {:?}", r.intent);

    // peer cap max=1: second peer blocks new profession
    let mut fire_rt2 = crate::FireFoodProfessionRuntime::default();
    let mut inp2 = inp;
    inp2.peer_count = 1.0;
    let r2 = late_make_fire_food_scan_tick(&tiles, &inp2, &mut fire_rt2);
    assert!(!r2.had_action, "peer cap should block late fire food");
}

#[test]
fn handling_fire_scan_winter_kindling_on_fire82() {
    use crate::FIRE;
    let tiles = vec![
        ScanTile::simple(FIRE, 1, 0),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_winter = true;
    inp.peer_count = 0.0;
    let mut fire_keeper = crate::FireKeeperProfessionRuntime {
        is_last_fire_keeper: true,
        weight: 1.0,
        ..Default::default()
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut fire_keeper,
        &mut fire_rt,
        &mut baker_rt,
        &mut baker_task,
    );
    assert!(r.had_action, "winter Fire82 should act {:?}", r.intent);
    // Kindling shortCraft on fire → UseAt or Seek/Craft
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt { .. }
                | ShortCraftLiveIntent::SeekOrCraft { .. }
                | ShortCraftLiveIntent::CraftItem { .. }
                | ShortCraftLiveIntent::DropAt { .. }
        ),
        "unexpected {:?}",
        r.intent
    );
    assert!(fire_keeper.fire_place_touched);
}

#[test]
fn apply_profession_scan_tick_writes_fire_place_sticky() {
    // FIRE-PLACE-STICKY: isHandlingFire GetCloseFire → Player.ai_fire_place_*
    use std::sync::Arc;
    use crate::FIRE;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(10, 10, FIRE);
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.home_x = 10;
    p.home_y = 10;
    p.x = 10;
    p.y = 10;
    p.age = 20.0;
    p.food = 5.0;
    p.fire_keeper_profession.is_last_fire_keeper = true;
    p.fire_keeper_profession.weight = 1.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let _ = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingFire,
        "MID_PRIORITY_TASKS",
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(p.ai_fire_place_id, FIRE, "GetCloseFire should stick Fire 82");
    assert_eq!((p.ai_fire_place_x, p.ai_fire_place_y), (10, 10));
}

#[test]
fn apply_profession_scan_tick_ashes_sticky_still_get_close_fire() {
    // Haxe: AiBase L1084–1100 GetCloseFire (not sticky ashes) then firePlace write
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(10, 10, 86); // Ashes on old firePlace tile
        w.set_object(12, 10, crate::FIRE);
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.home_x = 10;
    p.home_y = 10;
    p.x = 10;
    p.y = 10;
    p.age = 20.0;
    p.food = 5.0;
    p.ai_fire_place_id = 82;
    p.ai_fire_place_x = 10;
    p.ai_fire_place_y = 10;
    p.fire_keeper_profession.is_last_fire_keeper = true;
    p.fire_keeper_profession.weight = 1.0;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let _ = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingFire,
        "MID_PRIORITY_TASKS",
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.ai_fire_place_id,
        crate::FIRE,
        "GetCloseFire should take Fire 82 not sticky ashes"
    );
    assert_eq!((p.ai_fire_place_x, p.ai_fire_place_y), (12, 10));
}

#[test]
fn handling_fire_scan_uses_best_ai_flags_not_peer_heuristic() {
    // FIRE-BEST-AI: scan no longer ORs last/weight/peer_count; uses distance-pick flags
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_best_fire_keeper_at_home = false;
    inp.is_best_fire_keeper_at_fire = false;
    inp.peer_count = 0.0;
    let mut fire_keeper = crate::FireKeeperProfessionRuntime {
        is_last_fire_keeper: true,
        weight: 1.0,
        ..Default::default()
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut fire_keeper,
        &mut fire_rt,
        &mut baker_rt,
        &mut baker_task,
    );
    assert!(
        !r.had_action,
        "not-best should skip even with last/weight/no peers {:?}",
        r.intent
    );
    assert_eq!(fire_keeper.weight, 0.0);

    inp.is_best_fire_keeper_at_home = true;
    let mut fire_keeper2 = crate::FireKeeperProfessionRuntime::default();
    let r2 = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut fire_keeper2,
        &mut fire_rt,
        &mut baker_rt,
        &mut baker_task,
    );
    assert!(
        r2.had_action,
        "best at home with no fire should craft {:?}",
        r2.intent
    );
}

#[test]
fn is_self_best_fire_keeper_from_state_closer_weight_wins() {
    // Haxe: getBestAiForObjByProfession FIREKEEPER vs home; weight>0 + closer wins
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut far = crate::Player::new(1, 1, "far@t");
    far.home_x = 10;
    far.home_y = 10;
    far.x = 20;
    far.y = 10;
    far.age = 20.0;
    far.food = 5.0;
    let mut close = crate::Player::new(2, 2, "close@t");
    close.home_x = 10;
    close.home_y = 10;
    close.x = 11;
    close.y = 10;
    close.age = 20.0;
    close.food = 5.0;
    close.fire_keeper_profession.weight = 1.0;
    state.players.insert(1, far);
    state.players.insert(2, close);
    assert!(
        !is_self_best_fire_keeper_from_state(&state, 1, 10, 10, 10, 10),
        "far self without weight loses to closer FIREKEEPER"
    );
    assert!(
        is_self_best_fire_keeper_from_state(&state, 2, 10, 10, 10, 10),
        "closer weight wins"
    );
}

#[test]
fn apply_profession_scan_tick_far_peer_not_best_skips_new_fire() {
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut far = crate::Player::new(1, 1, "far@t");
    far.home_x = 10;
    far.home_y = 10;
    far.x = 20;
    far.y = 10;
    far.age = 20.0;
    far.food = 5.0;
    far.fire_keeper_profession.is_last_fire_keeper = true;
    let mut close = crate::Player::new(2, 2, "close@t");
    close.home_x = 10;
    close.home_y = 10;
    close.x = 11;
    close.y = 10;
    close.age = 20.0;
    close.food = 5.0;
    close.fire_keeper_profession.weight = 1.0;
    state.players.insert(1, far);
    state.players.insert(2, close);
    let hub = ol_net::OutboundHub::new();
    let _ = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingFire,
        "MID_PRIORITY_TASKS",
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.fire_keeper_profession.weight, 0.0,
        "getBestAi zeros self when another AI wins"
    );
}

#[test]
fn handling_fire_scan_hot_coals_make_fire_food_3_sets_r30() {
    // FIRE-CRAFT-R30: firePlace Hot Coals beyond near-player r=8 → makeFireFood(3) wrap
    use crate::HOT_COALS;
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 15, 0),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_best_fire_keeper_at_fire = true;
    inp.peer_count = 0.0;
    let mut fire_keeper = crate::FireKeeperProfessionRuntime {
        is_last_fire_keeper: true,
        weight: 1.0,
        ..Default::default()
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut fire_keeper,
        &mut fire_rt,
        &mut baker_rt,
        &mut baker_task,
    );
    assert!(r.had_action, "hot-coals makeFireFood(3) should act {:?}", r.intent);
    assert_eq!(
        fire_keeper.craft_search_radius_override,
        Some(crate::FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS)
    );
}

#[test]
fn apply_profession_scan_tick_hot_coals_craft_copies_r30() {
    // FIRE-CRAFT-R30: live scan copies wrap onto itemToCraft.maxSearchRadius then takes
    use std::sync::Arc;
    use crate::HOT_COALS;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    {
        let mut w = state.world.write().unwrap();
        w.set_object(15, 0, HOT_COALS);
    }
    let mut p = crate::Player::new(1, 1, "p@t");
    p.home_x = 0;
    p.home_y = 0;
    p.x = 0;
    p.y = 0;
    p.age = 20.0;
    p.food = 5.0;
    p.fire_keeper_profession.is_last_fire_keeper = true;
    p.fire_keeper_profession.weight = 1.0;
    assert_eq!(p.craft_ai.runtime.item.max_search_radius, 60);
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let _ = apply_profession_scan_tick(
        &mut state,
        &hub,
        1,
        ProfessionScanKind::HandlingFire,
        "MID_PRIORITY_TASKS",
    );
    let p = state.players.get(&1).unwrap();
    assert_eq!(
        p.craft_ai.runtime.item.max_search_radius,
        crate::FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS
    );
    assert_eq!(p.fire_keeper_profession.craft_search_radius_override, None);
}

#[test]
fn handling_fire_scan_fire82_tries_firewood() {
    // Haxe: AiBase L1195 shortCraftOnTarget(344, firePlace) even when none stocked
    use crate::FIRE;
    let tiles = vec![
        ScanTile::simple(FIRE, 1, 0),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.is_winter = false;
    inp.peer_count = 0.0;
    let mut fire_keeper = crate::FireKeeperProfessionRuntime {
        is_last_fire_keeper: true,
        weight: 1.0,
        ..Default::default()
    };
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut fire_keeper,
        &mut fire_rt,
        &mut baker_rt,
        &mut baker_task,
    );
    assert!(r.had_action, "Fire82 should try firewood {:?}", r.intent);
    assert!(
        matches!(
            r.intent,
            ShortCraftLiveIntent::UseAt { .. }
                | ShortCraftLiveIntent::SeekOrCraft { .. }
                | ShortCraftLiveIntent::CraftItem { .. }
                | ShortCraftLiveIntent::DropAt { .. }
        ),
        "unexpected {:?}",
        r.intent
    );
}

#[test]
fn peer_count_for_kind_handling_fire_is_fire_keeper_not_fire_food() {
    // Haxe: AiBase L1133 countProfession('FIREKEEPER') vs FIREFOODMAKER
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut food = crate::Player::new(1, 1, "food@t");
    food.home_x = 10;
    food.home_y = 10;
    food.x = 10;
    food.y = 10;
    food.age = 20.0;
    food.food = 5.0;
    food.fire_food_profession.is_last_fire_food = true;
    let mut keep = crate::Player::new(2, 2, "keep@t");
    keep.home_x = 10;
    keep.home_y = 10;
    keep.x = 11;
    keep.y = 10;
    keep.age = 20.0;
    keep.food = 5.0;
    keep.fire_keeper_profession.is_last_fire_keeper = true;
    let mut self_p = crate::Player::new(3, 3, "self@t");
    self_p.home_x = 10;
    self_p.home_y = 10;
    self_p.x = 12;
    self_p.y = 10;
    self_p.age = 20.0;
    self_p.food = 5.0;
    state.players.insert(1, food);
    state.players.insert(2, keep);
    state.players.insert(3, self_p);
    assert_eq!(
        peer_count_for_kind(ProfessionScanKind::HandlingFire, &state, 3, 10, 10),
        1.0
    );
    assert_eq!(
        peer_count_for_kind(ProfessionScanKind::FireFood, &state, 3, 10, 10),
        1.0
    );
}

#[test]
fn ladder_mid_empty_runs_late_make_fire_food() {
    use crate::baker_profession::RAW_MUTTON;
    use crate::HOT_COALS;
    // No fire place / oven for isHandlingFire early — late makeFireFood(1) should still cook
    let tiles = vec![
        ScanTile::simple(HOT_COALS, 5, 0), // outside near-player r=8 from (0,0)? 5 is within 8
        ScanTile::simple(RAW_MUTTON, 0, 1),
        ScanTile::empty(0, 0, 0, 0),
    ];
    // Coals at (5,0) is within r=8 so HandlingFire will take MakeFireFood(2) first — that's ok.
    // Use coals far for pure late residual: put coals outside near radius and home.
    let tiles_far = vec![
        ScanTile::simple(HOT_COALS, 15, 0),
        ScanTile::simple(RAW_MUTTON, 14, 0),
        ScanTile::simple(crate::FIRE, 16, 0),
        ScanTile::empty(0, 0, 0, 0),
    ];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.home_x = 15;
    inp.home_y = 0;
    inp.peer_count = 0.0;
    let sticky = ProfessionStickySnapshot {
        age: 20.0,
        ..Default::default()
    };
    let mut farm_task = FarmTaskState::default();
    let mut farm_rt = FarmProfessionRuntime::default();
    let mut smith_rt = SmithProfessionRuntime::default();
    let mut baker_rt = BakerProfessionRuntime::default();
    let mut baker_task = BakerTaskState::default();
    let mut shepherd_rt = crate::ShepherdProfessionRuntime::default();
    let mut pottery_rt = crate::PotterProfessionRuntime::default();
    let mut fire_rt = crate::FireFoodProfessionRuntime::default();
    let mut fire_keeper = crate::FireKeeperProfessionRuntime::default();
    let r = ladder_profession_scan_tick(
        PriorityRung::LowPriorityWork,
        &tiles_far,
        &inp,
        &sticky,
        &mut farm_task,
        &mut farm_rt,
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut fire_keeper,
        &mut crate::GraveKeeperProfessionRuntime::default(),
        &mut crate::HunterProfessionRuntime::default(),
        &mut crate::LumberjackProfessionRuntime::default(),
        &mut crate::CollectorProfessionRuntime::default(),
        &mut crate::FoodServerProfessionRuntime::default(),
    );
    // LowPriority age-rotated then late makeFireFood / makeStuff — should not panic
    let _ = r;
    // Direct late residual with home near coals
    let mut fire_rt2 = crate::FireFoodProfessionRuntime::default();
    let r2 = late_make_fire_food_scan_tick(&tiles, &inp, &mut fire_rt2);
    // With home at 15,0 and tiles at home-ish, late may act
    let mut inp_home = ProfessionScanInput::basic(15, 0, 0);
    inp_home.peer_count = 0.0;
    let mut fire_rt3 = crate::FireFoodProfessionRuntime::default();
    let r3 = late_make_fire_food_scan_tick(&tiles_far, &inp_home, &mut fire_rt3);
    assert!(
        r2.had_action || r3.had_action,
        "late fire food should find work on coals+mutton"
    );
}

#[test]
fn farm_action_defer_sheep_writes_basic_farmer_weight_sticky() {
    // AI-FARM-STICKY: DeferSheepHerding -> profession BASICFARMER=1 on farm_rt
    // Haxe: AiBase.doBasicFarming ~2400
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let mut farm_rt = FarmProfessionRuntime::default();
    assert!(farm_rt.weights.is_empty());
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::DeferSheepHerding {
            max_profession: 2,
        },
        &mut farm_rt,
    );
    assert_eq!(
        farm_rt.weights.get(&FarmProfession::BasicFarmer),
        Some(&1.0)
    );
    let _ = r;
    let r2 = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ClearBasicFarmerWeight,
        &mut farm_rt,
    );
    assert!(!r2.had_action);
    assert_eq!(
        farm_rt.weights.get(&FarmProfession::BasicFarmer),
        Some(&0.0)
    );
    assert_eq!(basic_farmer_weight_from_runtime(&farm_rt), 0.0);
}

#[test]
fn farm_action_clear_advanced_farmer_weight_sticky() {
    // Haxe: AiBase.doAdvancedFarming L4070 profession['ADVANCEDFARMER']=0
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let mut farm_rt = FarmProfessionRuntime::default();
    farm_rt
        .weights
        .insert(FarmProfession::AdvancedFarmer, 1.0);
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ClearAdvancedFarmerWeight,
        &mut farm_rt,
    );
    assert!(!r.had_action);
    assert_eq!(
        farm_rt.weights.get(&FarmProfession::AdvancedFarmer),
        Some(&0.0)
    );
}

#[test]
fn farm_action_defer_pottery_expands_do_pottery() {
    // Haxe: doPrepareRows L2214 doPottery(maxProfession)
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let mut farm_rt = FarmProfessionRuntime::default();
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::DeferPottery {
            max_profession: 2,
        },
        &mut farm_rt,
    );
    let _ = r;
}

#[test]
fn farm_profession_input_reads_player_basic_farmer_weight() {
    // AI-FARM-STICKY: Player.farm_profession weight read-through (default 1.0)
    use crate::Player;
    let mut p = Player::new(1, 1, "farm@t");
    assert_eq!(basic_farmer_weight_from_runtime(&p.farm_profession), 1.0);
    p.farm_profession
        .weights
        .insert(FarmProfession::BasicFarmer, 7.0);
    assert_eq!(basic_farmer_weight_from_runtime(&p.farm_profession), 7.0);
}

#[test]
fn farm_after_sheep_assigned_max_profession_pass_through() {
    // AI-FARM-STICKY: assigned doBasicFarming(100) -> doAdvancedFarming(100)
    // Late plant caps satisfied so after_sheep reaches DeferAdvancedFarming.
    use crate::farmer_profession::{DRY_PLANTED_CORN, DRY_PLANTED_WHEAT};
    let mut task = FarmTaskState {
        corn_planter: 0.0,
        ..Default::default()
    };
    let mut counts = crate::FarmCounts::default();
    counts.set(DRY_PLANTED_WHEAT, 30);
    counts.set(DRY_PLANTED_CORN, 12);
    let late = crate::do_basic_farming_after_sheep(&counts, &mut task, 25.0, 100);
    assert_eq!(
        late,
        FarmAction::DeferAdvancedFarming {
            max_profession: 100
        }
    );
}

// C-SS-MIN-AGE-AI: live MinAgeToEat on age-job / sticky job sensor flags
#[test]
fn age_job_pending_ex_live_min_age() {
    let mid = ProfessionStickySnapshot {
        age: 4.0,
        ..Default::default()
    };
    assert!(mid.age_job_pending()); // default min 3
    assert!(!mid.age_job_pending_ex(5.0));
    assert!(mid.age_job_pending_ex(3.0));
    assert!(mid.age_job_pending_ex(4.0)); // age >= min
}

#[test]
fn job_sensor_flags_from_sticky_ex_live_min_age() {
    let mid = ProfessionStickySnapshot {
        age: 4.0,
        baker_assigned: true,
        ..Default::default()
    };
    let f = job_sensor_flags_from_sticky_ex(&mid, 5.0);
    assert!(f.has_assigned_job);
    assert!(!f.age_job_pending);
    let f2 = job_sensor_flags_from_sticky_ex(&mid, 3.0);
    assert!(f2.age_job_pending);
}

// AI-JOB-SMITH-RESID: multi-prof npc peer_count from snapshot rows
#[test]
fn npc_peer_count_for_kind_multi_prof_and_wounded() {
    // Haxe: countProfession per lastProfession; skip wounded / other home
    let rows = [
        NpcProfessionPeerRow {
            conn_id: 1,
            home_x: 10,
            home_y: 10,
            age: 25.0,
            food_store: 5.0,
            deleted: false,
            has_player_to_follow: false,
            is_wounded: false,
            last_is_smith: true,
            last_is_baker: false,
            last_is_potter: false,
            last_is_shepherd: false,
            last_is_farm: false,
            last_farm: None,
            last_is_fire_food: false,
            last_is_fire_keeper: false,
            last_is_hunter: false,
            last_is_lumberjack: false,
            last_is_collector: false,
            last_is_foodserver: false,
            last_is_tailor: false,
        },
        NpcProfessionPeerRow {
            conn_id: 2,
            home_x: 10,
            home_y: 10,
            age: 30.0,
            food_store: 5.0,
            deleted: false,
            has_player_to_follow: false,
            is_wounded: true, // wounded smith excluded
            last_is_smith: true,
            last_is_baker: false,
            last_is_potter: false,
            last_is_shepherd: false,
            last_is_farm: false,
            last_farm: None,
            last_is_fire_food: false,
            last_is_fire_keeper: false,
            last_is_hunter: false,
            last_is_lumberjack: false,
            last_is_collector: false,
            last_is_foodserver: false,
            last_is_tailor: false,
        },
        NpcProfessionPeerRow {
            conn_id: 3,
            home_x: 10,
            home_y: 10,
            age: 28.0,
            food_store: 5.0,
            deleted: false,
            has_player_to_follow: false,
            is_wounded: false,
            last_is_smith: false,
            last_is_baker: true,
            last_is_potter: false,
            last_is_shepherd: false,
            last_is_farm: true,
            last_farm: Some(FarmProfession::BasicFarmer),
            last_is_fire_food: false,
            last_is_fire_keeper: true,
            last_is_hunter: false,
            last_is_lumberjack: false,
            last_is_collector: false,
            last_is_foodserver: false,
            last_is_tailor: true,
        },
        NpcProfessionPeerRow {
            conn_id: 4,
            home_x: 99,
            home_y: 99,
            age: 28.0,
            food_store: 5.0,
            deleted: false,
            has_player_to_follow: false,
            is_wounded: false,
            last_is_smith: true,
            last_is_baker: true,
            last_is_potter: true,
            last_is_shepherd: true,
            last_is_farm: true,
            last_farm: Some(FarmProfession::WaterBringer),
            last_is_fire_food: true,
            last_is_fire_keeper: false,
            last_is_hunter: false,
            last_is_lumberjack: false,
            last_is_collector: false,
            last_is_foodserver: false,
            last_is_tailor: false,
        },
    ];
    // Self=99 at home 10,10: one healthy smith (conn 1), wounded excluded, other-home excluded
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Smith, &rows, 99, 10, 10, 3.0, 60.0),
        1.0
    );
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Baker, &rows, 99, 10, 10, 3.0, 60.0),
        1.0
    );
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Farm, &rows, 99, 10, 10, 3.0, 60.0),
        1.0
    );
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Pottery, &rows, 99, 10, 10, 3.0, 60.0),
        0.0
    );
    // Haxe: countProfession('FIREKEEPER') ≠ FIREFOODMAKER (conn 3 last_is_fire_keeper)
    assert_eq!(
        npc_peer_count_for_kind(
            ProfessionScanKind::HandlingFire,
            &rows,
            99,
            10,
            10,
            3.0,
            60.0
        ),
        1.0
    );
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::FireFood, &rows, 99, 10, 10, 3.0, 60.0),
        0.0
    );
    // Includes self-home baker-row marked tailor (conn 3); other-home excluded
    assert_eq!(
        count_tailor_profession_from_rows(&rows, 10, 10, 3.0, 60.0),
        1
    );
    // Self=1 excludes self smith
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Smith, &rows, 1, 10, 10, 3.0, 60.0),
        0.0
    );
    // Job-specific farm lasts: conn 3 BASICFARMER at home; conn 4 other home excluded
    assert_eq!(
        farm_peer_lasts_from_npc_rows(&rows, 99, 10, 10, 3.0, 60.0),
        Some(vec![FarmProfession::BasicFarmer])
    );
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.peer_count = 9.0;
    inp.peer_count_by_kind = Some(npc_peer_counts_by_kind(&rows, 99, 10, 10, 3.0, 60.0));
    assert_eq!(
        inp.peer_count_for_kind(ProfessionScanKind::Smith),
        npc_peer_count_for_kind(ProfessionScanKind::Smith, &rows, 99, 10, 10, 3.0, 60.0)
    );
    assert_eq!(
        inp.peer_count_for_kind(ProfessionScanKind::Baker),
        npc_peer_count_for_kind(ProfessionScanKind::Baker, &rows, 99, 10, 10, 3.0, 60.0)
    );
    assert_ne!(
        inp.peer_count_for_kind(ProfessionScanKind::Smith),
        inp.peer_count
    );
}

#[test]
fn count_profession_gravekeeper_skips_max_age_gate() {
    // Haxe: AiBase.countProfession L1294 age > MaxAge-2 && profession != 'GRAVEKEEPER'
    assert!(count_profession_applies_max_age_skip("SMITH"));
    assert!(count_profession_applies_max_age_skip("FIREKEEPER"));
    assert!(!count_profession_applies_max_age_skip("GRAVEKEEPER"));
    assert!(!count_profession_applies_max_age_skip("gravekeeper"));
    let old = NpcProfessionPeerRow {
        conn_id: 2,
        home_x: 10,
        home_y: 10,
        age: 59.0,
        food_store: 5.0,
        ..NpcProfessionPeerRow::default()
    };
    assert!(!old.eligible(1, 10, 10, 3.0, 60.0));
    assert!(old.eligible_ex(1, 10, 10, 3.0, 60.0, true));
    let follow = NpcProfessionPeerRow {
        conn_id: 3,
        home_x: 10,
        home_y: 10,
        age: 20.0,
        food_store: 5.0,
        has_player_to_follow: true,
        ..NpcProfessionPeerRow::default()
    };
    assert!(!follow.eligible(1, 10, 10, 3.0, 60.0));
    let hungry = NpcProfessionPeerRow {
        conn_id: 4,
        home_x: 10,
        home_y: 10,
        age: 20.0,
        food_store: -0.1,
        ..NpcProfessionPeerRow::default()
    };
    assert!(!hungry.eligible(1, 10, 10, 3.0, 60.0));
}

#[test]
fn peer_counts_by_kind_from_state_splits_smith_and_baker() {
    // SMITH-LADDER-PEER-KIND: player/sim table, not primary-kind-only
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut mk = |conn: u64| {
        let mut p = crate::Player::new(conn as i32, conn, "p@t");
        p.home_x = 10;
        p.home_y = 10;
        p.age = 20.0;
        p.food = 5.0;
        p.deleted = false;
        p
    };
    let mut a = mk(1);
    a.smith_profession.is_last_smith = true;
    let mut b = mk(2);
    b.baker_profession.is_last_baker = true;
    let mut c = mk(3);
    c.smith_profession.is_last_smith = true;
    c.baker_profession.is_last_baker = true;
    state.players.insert(1, a);
    state.players.insert(2, b);
    state.players.insert(3, c);
    let tbl = peer_counts_by_kind_from_state(&state, 1, 10, 10);
    let get = |k: ProfessionScanKind| {
        tbl.iter()
            .find(|(kk, _)| *kk == k)
            .map(|(_, c)| *c)
            .unwrap_or(-1.0)
    };
    assert_eq!(
        get(ProfessionScanKind::Smith),
        1.0,
        "self excluded; only conn 3 smith, got {tbl:?}"
    );
    assert_eq!(
        get(ProfessionScanKind::Baker),
        2.0,
        "conn 2+3 bakers, got {tbl:?}"
    );
    assert_eq!(get(ProfessionScanKind::Pottery), 0.0);
    let mut inp = ProfessionScanInput::basic(10, 10, 0);
    inp.peer_count = 99.0;
    inp.peer_count_by_kind = Some(tbl);
    assert_eq!(inp.peer_count_for_kind(ProfessionScanKind::Smith), 1.0);
    assert_eq!(inp.peer_count_for_kind(ProfessionScanKind::Baker), 2.0);
    assert_ne!(
        inp.peer_count_for_kind(ProfessionScanKind::Smith),
        inp.peer_count
    );
}

#[test]
fn clothing_has_tailor_for_player_respects_peer_cap() {
    // CLOTHING-HAS-TAILOR: hasOrBecomeProfession('TAILOR') not hardcoded true
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let mut mk = |conn: u64| {
        let mut p = crate::Player::new(conn as i32, conn, "p@t");
        p.home_x = 10;
        p.home_y = 10;
        p.age = 20.0;
        p.food = 5.0;
        p.deleted = false;
        p
    };
    let mut tailor = mk(1);
    tailor.last_profession = Some("TAILOR".into());
    let mut smith = mk(2);
    smith.last_profession = Some("SMITH".into());
    let idle = mk(3);
    state.players.insert(1, tailor);
    state.players.insert(2, smith);
    state.players.insert(3, idle);
    assert_eq!(count_tailor_profession_at_home(&state, 10, 10), 1);
    assert!(clothing_has_tailor_for_player(
        &state,
        state.players.get(&1).unwrap()
    ));
    assert!(
        !clothing_has_tailor_for_player(&state, state.players.get(&2).unwrap()),
        "sticky SMITH was_idle=0; count 1 >= max 1"
    );
    assert!(
        clothing_has_tailor_for_player(&state, state.players.get(&3).unwrap()),
        "idle was_idle=1 expands cap to 2"
    );
    let mut assigned = mk(4);
    assigned.assigned_profession = Some("TAILOR".into());
    state.players.insert(4, assigned);
    assert!(
        clothing_has_tailor_for_player(&state, state.players.get(&4).unwrap()),
        "assigned TAILOR uses max=100"
    );
}

#[test]
fn apply_clothing_craft_tick_assigns_last_tailor_when_gate_opens() {
    // Haxe hasOrBecomeProfession assigns lastProfession after high/quiver skip
    use std::sync::Arc;
    let mut state = crate::SimState::with_default_empty(Arc::new(ol_content::ContentDb::default()));
    let hub = ol_net::OutboundHub::new();
    let mut mk = |conn: u64| {
        let mut p = crate::Player::new(conn as i32, conn, "p@t");
        p.home_x = 10;
        p.home_y = 10;
        p.age = 20.0;
        p.food = 5.0;
        p.deleted = false;
        // Fill bottom + back so high reed-skirt and fillUpQuiver empty-quiver skip
        p.set_clothing_index_helper(4, Some(ol_world::NestedHelper::id_only(128)));
        p.set_clothing_index_helper(5, Some(ol_world::NestedHelper::id_only(198)));
        p
    };
    let mut tailor = mk(1);
    tailor.last_profession = Some("TAILOR".into());
    let mut smith = mk(2);
    smith.last_profession = Some("SMITH".into());
    let idle = mk(3);
    state.players.insert(1, tailor);
    state.players.insert(2, smith);
    state.players.insert(3, idle);
    let _ = apply_clothing_craft_tick(&mut state, &hub, 3);
    assert_eq!(
        state.players.get(&3).unwrap().last_profession.as_deref(),
        Some("TAILOR")
    );
    let r = apply_clothing_craft_tick(&mut state, &hub, 2);
    assert!(matches!(r, ShortCraftLiveApplyResult::Failed));
    assert_eq!(
        state.players.get(&2).unwrap().last_profession.as_deref(),
        Some("SMITH")
    );
}

// AI-JOB-LIVE-IO-RESID: farm hasOrBecomeProfession live peer-cap
#[test]
fn farm_scan_peer_count_job_specific_vs_aggregate() {
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.peer_count = 5.0;
    assert_eq!(
        farm_scan_peer_count(&inp, FarmProfession::BasicFarmer),
        5.0
    );
    inp.farm_peer_lasts = Some(vec![
        FarmProfession::BasicFarmer,
        FarmProfession::WaterBringer,
        FarmProfession::WaterBringer,
        FarmProfession::BerryFarmer,
    ]);
    assert_eq!(
        farm_scan_peer_count(&inp, FarmProfession::BasicFarmer),
        1.0
    );
    assert_eq!(
        farm_scan_peer_count(&inp, FarmProfession::WaterBringer),
        2.0
    );
    assert_eq!(
        farm_scan_peer_count(&inp, FarmProfession::CarrotFarmer),
        0.0
    );
}

#[test]
fn farm_profession_scan_tick_peer_cap_skips_when_not_sticky() {
    // Pull-carrots tile would otherwise yield ShortCraft if profession allowed.
    let tiles = vec![ScanTile::simple(400, 1, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.was_idle = 0.0;
    inp.farm_peer_lasts = Some(vec![
        FarmProfession::BasicFarmer,
        FarmProfession::BasicFarmer,
    ]);
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::BasicFarmer),
        "AGE_ROTATED_JOB",
        &mut task,
        true,
        &mut rt,
    );
    assert!(!r.had_action, "max=2 with 2 BASICFARMER peers must skip, got {:?}", r.intent);
    assert!(rt.last_profession.is_none());

    // Sticky last BASICFARMER ignores peer cap (Haxe hasOrBecome).
    rt.last_profession = Some(FarmProfession::BasicFarmer);
    let r2 = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::BasicFarmer),
        "AGE_ROTATED_JOB",
        &mut task,
        true,
        &mut rt,
    );
    assert!(r2.had_action, "sticky last must farm despite full peer cap");
}

#[test]
fn farm_profession_scan_tick_room_becomes_and_assigns_last() {
    let tiles = vec![ScanTile::simple(400, 1, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.was_idle = 0.0;
    inp.farm_peer_lasts = Some(vec![FarmProfession::BasicFarmer]);
    let mut task = FarmTaskState::default();
    let mut rt = FarmProfessionRuntime::default();
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        Some(FarmProfession::BasicFarmer),
        "AGE_ROTATED_JOB",
        &mut task,
        false,
        &mut rt,
    );
    assert!(r.had_action);
    assert_eq!(rt.last_profession, Some(FarmProfession::BasicFarmer));
}

#[test]
fn attack_player_action_maps_kill_and_pickup() {
    use crate::attack_player::{AttackPlayerAction, GetWeaponAction, KNIFE};
    assert_eq!(
        attack_player_action_to_live_intent(
            AttackPlayerAction::Kill {
                target_p_id: 7,
                tx: 4,
                ty: 5,
            },
            KNIFE
        ),
        ShortCraftLiveIntent::Kill {
            target_p_id: 7,
            x: 4,
            y: 5
        }
    );
    assert_eq!(
        attack_player_action_to_live_intent(
            AttackPlayerAction::GetWeapon(GetWeaponAction::Pickup {
                x: 2,
                y: 0,
                id: KNIFE
            }),
            0
        ),
        ShortCraftLiveIntent::UseAt {
            x: 2,
            y: 0,
            target_id: KNIFE,
            actor_id: 0
        }
    );
}

#[test]
fn apply_profession_scan_from_sensors_attack_player_kills_adjacent() {
    use crate::attack_player::KNIFE;
    use ol_content::{ContentDb, ObjectDef};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    db.objects.insert(
        KNIFE,
        ObjectDef {
            id: KNIFE,
            description: "Knife".into(),
            name: "Knife".into(),
            deadly_distance: 1.5,
            ..ObjectDef::empty(KNIFE)
        },
    );
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    let mut a = crate::Player::new(1, 1, "atk@t");
    a.x = 10;
    a.y = 10;
    a.age = 20.0;
    a.food = 10.0;
    a.angry_time = 0.0;
    a.set_held(KNIFE, 0);
    a.last_attacked_player_id = 2;
    let mut b = crate::Player::new(2, 2, "tgt@t");
    b.x = 11;
    b.y = 10;
    b.age = 20.0;
    b.food = 10.0;
    b.angry_time = 0.0;
    b.set_held(KNIFE, 0);
    b.last_player_attacked_me_id = 1;
    state.players.insert(1, a);
    state.players.insert(2, b);
    let hub = ol_net::OutboundHub::new();
    let (rung, r) = apply_profession_scan_from_sensors(&mut state, &hub, 1, false, false);
    assert_eq!(rung, PriorityRung::Combat);
    assert!(
        matches!(
            r,
            ShortCraftLiveApplyResult::Dropped
                | ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Kill { .. })
        ),
        "combat rung should KILL adjacent armed target {:?}",
        r
    );
    let atk = state.players.get(&1).unwrap();
    assert!(atk.kill_mode || state.combat.wound_of(2) > 0 || state.players.get(&2).map(|p| p.deleted).unwrap_or(false));
}

#[test]
fn kill_animal_prefix_wolf_at_home_becomes_hunter() {
    // Haxe L5878–5898: passedTime>10, wolf 418 at home r=20, hasOrBecome HUNTER
    use crate::kill_animal::{KillAnimalPrefixKind, WOLF};
    use ol_content::{ContentDb, ObjectDef};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    db.objects.insert(WOLF, ObjectDef::empty(WOLF));
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    state.tick = 200;
    let mut p = crate::Player::new(1, 1, "hunt@t");
    p.age = 20.0;
    p.food = 10.0;
    p.home_x = 10;
    p.home_y = 10;
    p.x = 10;
    p.y = 10;
    p.ai_time_looked_for_deadly_animal_at_home = -1.0;
    state.players.insert(1, p);
    state.world.write().unwrap().set_object(14, 11, WOLF);
    let kind = apply_kill_animal_prefix_tick(&mut state, 1);
    assert_eq!(kind, KillAnimalPrefixKind::Continue);
    let p = state.players.get(&1).unwrap();
    assert_eq!(p.ai_animal_target_id, WOLF);
    assert_eq!((p.ai_animal_target_x, p.ai_animal_target_y), (14, 11));
    assert!(p.hunter_profession.is_last_hunter);
    assert_eq!(p.last_profession.as_deref(), Some("HUNTER"));
}

#[test]
fn kill_animal_body_food_below_zero_fails_tick() {
    // Haxe L5902: food_store < 0 return false (after prefix Skip/Continue)
    use crate::kill_animal::WOLF;
    use ol_content::{ContentDb, ObjectDef};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    db.objects.insert(WOLF, ObjectDef::empty(WOLF));
    let mut state = crate::SimState::with_default_empty(Arc::new(db));
    state.tick = 200;
    let mut p = crate::Player::new(1, 1, "hunt@t");
    p.age = 20.0;
    p.food = -0.5;
    p.home_x = 10;
    p.home_y = 10;
    p.x = 10;
    p.y = 10;
    p.ai_animal_target_id = WOLF;
    p.ai_animal_target_x = 12;
    p.ai_animal_target_y = 10;
    p.hunter_profession.is_last_hunter = true;
    state.players.insert(1, p);
    let hub = ol_net::OutboundHub::new();
    let r = apply_kill_animal_tick(&mut state, &hub, 1);
    assert!(
        matches!(r, ShortCraftLiveApplyResult::Failed),
        "hungry killAnimal must return false {:?}",
        r
    );
}

#[test]
fn do_critical_stuff_place_floor_under_home() {
    // Haxe L6091: placeFloorUnder(home) → shortCraft(881, parent, false)
    use crate::cleanup_profession::CUT_STONES;
    use ol_content::{ContentDb, ObjectDef};
    use std::sync::Arc;
    let mut db = ContentDb::default();
    db.allow_floor_placement.insert(237);
    db.objects.insert(237, ObjectDef::empty(237));
    db.objects.insert(CUT_STONES, ObjectDef::empty(CUT_STONES));
    let tiles = vec![ScanTile::simple(237, 0, 0)];
    let mut inp = ProfessionScanInput::basic(0, 0, 0);
    inp.content = Some(Arc::new(db));
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        PLACE_FLOOR_UNDER_RUNG,
        &mut FarmTaskState::default(),
        true,
        &mut FarmProfessionRuntime::default(),
    );
    assert!(r.had_action, "placeFloorUnder home should act {:?}", r.intent);
}

#[test]
fn do_critical_stuff_cleanup_empties_charcoal_basket() {
    // Haxe L6101: if (cleanUp()) return true — Basket of Charcoal 298
    use crate::cleanup_profession::{CRITICAL_CLEANUP_RUNG, BASKET_OF_CHARCOAL};
    let tiles = vec![ScanTile::simple(BASKET_OF_CHARCOAL, 2, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = farm_profession_scan_tick(
        &tiles,
        &inp,
        None,
        CRITICAL_CLEANUP_RUNG,
        &mut FarmTaskState::default(),
        true,
        &mut FarmProfessionRuntime::default(),
    );
    assert!(r.had_action, "cleanUp charcoal basket should act {:?}", r.intent);
}

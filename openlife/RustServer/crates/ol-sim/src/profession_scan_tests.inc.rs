// Tests for profession_scan (included into short_craft_intent::profession_scan).
use super::*;
use crate::baker_profession::{
    BakeAction, BakerProfessionRuntime, BakerTaskState, HOT_OVEN, RAW_MUTTON,
};
use crate::farmer_profession::{
    basic_farmer_weight_from_runtime, FarmAction, FarmProfession, FarmProfessionRuntime,
    FarmTaskState, BOWL_OF_SOIL, DRY_PLANTED_CARROTS, DYING_BUSH,
};
use crate::short_craft_intent::ShortCraftLiveIntent;
use crate::smith_profession::{
    SmithAction, SmithProfessionRuntime, BIG_CHARCOAL_PILE, FIRED_BOWL_TONGS, FIRED_NOZZLE_TONGS,
    FIRING_FORGE, FIRING_KILN, HOT_IRON_BLOOM_FLAT, SMITHING_HAMMER, WET_CLAY_BOWL, WET_CLAY_NOZZLE,
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
fn scan_world_radius_includes_empty_and_objects() {
    let w = mock_world_with(&[(DYING_BUSH, 5, 5), (663, 8, 5)]);
    let tiles = scan_world_radius(&w, None, 5, 5, 3);
    assert!(tiles
        .iter()
        .any(|t| t.parent_id == DYING_BUSH && t.x == 5 && t.y == 5));
    assert!(tiles.iter().any(|t| t.parent_id == 663));
    assert!(tiles.iter().any(|t| t.parent_id == 0));
    assert!(tiles.len() >= 7 * 7);
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
fn farm_short_craft_missing_target_seeks() {
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
    assert!(r.had_action);
    assert_eq!(
        r.intent,
        ShortCraftLiveIntent::SeekOrCraft {
            actor: DYING_BUSH,
            craft_if_needed: false,
        }
    );
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
    assert_eq!(r3.intent, ShortCraftLiveIntent::DeferPottery);
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
fn baker_defer_farm_had_action_no_use() {
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let r = bake_action_to_live_intent(&tiles, &inp, BakeAction::DeferFarm);
    assert!(r.had_action);
    assert_eq!(r.intent, ShortCraftLiveIntent::None);
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
    // AI-JOB-SMITH-RESID: early sticky + CRITICAL_CRAFT shortCraft tails
    assert_eq!(crit.len(), 2);
    assert_eq!(crit[0].rung_label, "EARLY_STICKY_SMITH");
    assert_eq!(crit[1].rung_label, "CRITICAL_CRAFT");
    let crit_open = plan_profession_ladder_steps(
        PriorityRung::CriticalCraft,
        &ProfessionStickySnapshot {
            age: 20.0,
            ..Default::default()
        },
    );
    assert_eq!(crit_open.len(), 1);
    assert_eq!(crit_open[0].rung_label, "CRITICAL_CRAFT");

    let idle = plan_profession_ladder_steps(PriorityRung::Escape, &sticky);
    assert!(idle.is_empty());
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
    );
    assert_eq!(
        intent,
        ShortCraftLiveIntent::Wait,
        "moving dropOnStart must Wait, got {intent:?}"
    );
    assert!(crate::drop_held_live_intent_actionable(intent));
    assert!(crate::live_intent_is_wait(intent));
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
        &farm, &smith, &baker, None, None, Some(&fire), None, 25.0,
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

    let hungry = plan_profession_ladder_steps(PriorityRung::ConsiderMakeFood, &sticky);
    assert_eq!(hungry.len(), 1);
    assert_eq!(hungry[0].kind, ProfessionScanKind::HandlingFire);
    assert_eq!(hungry[0].rung_label, "CONSIDER_MAKE_FOOD");

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
            last_is_fire_food: false,
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
            last_is_fire_food: false,
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
            last_is_fire_food: false,
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
            last_is_fire_food: true,
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
    // Self=1 excludes self smith
    assert_eq!(
        npc_peer_count_for_kind(ProfessionScanKind::Smith, &rows, 1, 10, 10, 3.0, 60.0),
        0.0
    );
}

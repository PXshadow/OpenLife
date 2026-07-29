// AI-HANDLING-FIRE: live scan helpers included from profession_scan.rs

/// Fire-keeper / isHandlingFire: sensors → pure body → live intent.
// Haxe: isHandlingFire / FIREKEEPER assigned|last ~730; mid ~634; temp ~1740; hungry ~8540
pub fn handling_fire_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    fire_keeper: &mut crate::FireKeeperProfessionRuntime,
    fire_food_rt: &mut crate::FireFoodProfessionRuntime,
    baker_rt: &mut BakerProfessionRuntime,
    baker_task: &mut BakerTaskState,
) -> ProfessionScanTickResult {
    let map: Vec<crate::HandlingFireMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::HandlingFireMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    // Haxe: getBestAiForObjByProfession sticky/weight/peer heuristic (multi-peer dist residual)
    let is_best = fire_keeper.weight > 0.0
        || fire_keeper.is_last_fire_keeper
        || fire_keeper.is_assigned_fire_keeper
        || inp.peer_count < 1.0;
    // Haxe: TimeHelper.Season == Winter → kindling first on Fire 82
    let sensors = crate::handling_fire_sensors_from_map(
        &map,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.is_winter,
        inp.target_reachable,
        false,
        is_best,
        is_best,
        inp.peer_count,
        inp.was_idle,
    );
    let fire_map: Vec<crate::FireFoodMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::FireFoodMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    let mut fire_counts = crate::fill_fire_food_counts_from_map(
        inp.home_x,
        inp.home_y,
        inp.held_id,
        &fire_map,
        crate::FIRE_FOOD_HOME_RADIUS,
        inp.is_hungry,
        false,
        inp.has_bean_seeds,
    );
    fire_counts.has_corn_seeds = tiles.iter().any(|t| {
        matches!(
            t.parent_id,
            crate::BOWL_CORN_KERNELS
                | crate::shepherd_profession::DRIED_EAR_OF_CORN
                | crate::shepherd_profession::BOWL_CORN_COB
        )
    });
    let Some(action) = crate::try_decide_handling_fire_from_rung(
        inp.profession_is_sticky,
        rung_label,
        inp.is_assigned_job,
        &sensors,
        fire_keeper,
        &fire_counts,
        fire_food_rt,
        inp.peer_count,
        inp.was_idle,
    ) else {
        return ProfessionScanTickResult::none();
    };
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    // Haxe: hotOven near → doBaking(2) nested expand (isHandlingFire ~1091–1093)
    if let crate::HandlingFireAction::DoBaking { max_people } = action {
        return expand_handling_fire_do_baking(tiles, inp, baker_rt, baker_task, max_people);
    }
    handling_fire_action_to_live_intent(tiles, inp, action)
}

/// Nested doBaking(max) from isHandlingFire hot-oven gate.
// Haxe: AiBase.isHandlingFire doBaking(2) ~1093
fn expand_handling_fire_do_baking(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    baker_rt: &mut BakerProfessionRuntime,
    baker_task: &mut BakerTaskState,
    max_people: i32,
) -> ProfessionScanTickResult {
    let map = bake_map_from_scan(tiles);
    let held_uses = if inp.held_uses > 0 { inp.held_uses } else { 1 };
    let origin_floor = floor_at_scan(tiles, inp.home_x, inp.home_y);
    let bake_counts = fill_bake_counts_from_map_ex(
        inp.home_x,
        inp.home_y,
        inp.held_id,
        held_uses,
        &map,
        OVEN_SEARCH_RADIUS,
        inp.is_hungry,
        inp.has_carrot_seeds,
        inp.has_bean_seeds,
        origin_floor,
    );
    let action = crate::do_baking(
        &bake_counts,
        baker_rt,
        baker_task,
        max_people,
        inp.peer_count,
        inp.was_idle,
        0,
    );
    if matches!(action, BakeAction::None | BakeAction::Abort) {
        // Oven near but bake empty → still seek hot oven (staging)
        return ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekOrCraft {
                actor: crate::HOT_ADOBE_OVEN,
                craft_if_needed: false,
            },
            had_action: true,
        };
    }
    bake_action_to_live_intent(tiles, inp, action)
}

/// Map HandlingFireAction → live shortCraft intent.
// Haxe: isHandlingFire shortCraftOnTarget / craftItem / useHeld / GetOrCraft / makeFireFood
pub fn handling_fire_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: crate::HandlingFireAction,
) -> ProfessionScanTickResult {
    use crate::HandlingFireAction;
    match action {
        HandlingFireAction::None => ProfessionScanTickResult::none(),
        HandlingFireAction::MakeFireFood { max_people } => {
            let _ = max_people;
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::SeekOrCraft {
                    actor: crate::HOT_COALS,
                    craft_if_needed: true,
                },
                had_action: true,
            }
        }
        // Prefer expand_handling_fire_do_baking from scan tick; fallback seek oven.
        HandlingFireAction::DoBaking { max_people } => {
            let _ = max_people;
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::SeekOrCraft {
                    actor: crate::HOT_ADOBE_OVEN,
                    craft_if_needed: false,
                },
                had_action: true,
            }
        }
        HandlingFireAction::CraftItem { object_id }
        | HandlingFireAction::GetOrCraft { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        HandlingFireAction::UseHeldOnFire { fire_object_id } => fire_food_action_to_live_intent(
            tiles,
            inp,
            crate::FireFoodAction::ShortCraft {
                actor: inp.held_id,
                target: fire_object_id,
            },
        ),
        HandlingFireAction::ShortCraftOnFire {
            actor,
            fire_object_id,
        } => {
            if actor == 0 {
                return fire_food_action_to_live_intent(
                    tiles,
                    inp,
                    crate::FireFoodAction::ShortCraftOnGround {
                        target: fire_object_id,
                    },
                );
            }
            fire_food_action_to_live_intent(
                tiles,
                inp,
                crate::FireFoodAction::ShortCraft {
                    actor,
                    target: fire_object_id,
                },
            )
        }
    }
}

/// Late residual makeFireFood(1) after makeStuff / critical / hungry paths.
// Haxe: doTimeStuffHelper makeFireFood(1) ~833; doCriticalStuff ~6107; hungry ~8594/8603
pub fn late_make_fire_food_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    fire_rt: &mut crate::FireFoodProfessionRuntime,
) -> ProfessionScanTickResult {
    let map: Vec<crate::FireFoodMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::FireFoodMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    let mut counts = crate::fill_fire_food_counts_from_map(
        inp.home_x,
        inp.home_y,
        inp.held_id,
        &map,
        crate::FIRE_FOOD_HOME_RADIUS,
        inp.is_hungry,
        false,
        inp.has_bean_seeds,
    );
    counts.has_corn_seeds = tiles.iter().any(|t| {
        matches!(
            t.parent_id,
            crate::BOWL_CORN_KERNELS
                | crate::shepherd_profession::DRIED_EAR_OF_CORN
                | crate::shepherd_profession::BOWL_CORN_COB
        )
    });
    // Haxe: late ~833 / hungry ~8594 / critical ~6107 all use maxPeople=1
    let path = if inp.is_hungry {
        crate::FireFoodDispatchPath::Hungry
    } else {
        crate::FireFoodDispatchPath::Late
    };
    let action =
        crate::make_fire_food_late_or_hungry(&counts, fire_rt, path, inp.peer_count, inp.was_idle);
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    fire_food_action_to_live_intent(tiles, inp, action)
}

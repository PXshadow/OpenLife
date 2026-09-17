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
    // Haxe: getBestAiForObjByProfession('FIREKEEPER', home|firePlace) distance pick
    // Haxe: TimeHelper.Season == Winter → kindling first on Fire 82
    // Haxe: AiBase L1084–1085 GetCloseFire then myPlayer.firePlace = firePlace (no sticky)
    // Haxe: L1133 hasOrBecomeProfession('FIREKEEPER', 3) uses countProfession('FIREKEEPER')
    let fire_keeper_peers = inp.peer_count_for_kind(ProfessionScanKind::HandlingFire);
    let fire_food_peers = inp.peer_count_for_kind(ProfessionScanKind::FireFood);
    let sensors = crate::handling_fire_sensors_from_map_ex(
        &map,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.is_winter,
        inp.target_reachable,
        false,
        inp.is_best_fire_keeper_at_home,
        inp.is_best_fire_keeper_at_fire,
        fire_keeper_peers,
        inp.was_idle,
        None,
    );
    fire_keeper.fire_place_touched = true;
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
    fire_counts.apply_popcorn_stock_from_map(
        inp.home_x,
        inp.home_y,
        inp.player_x,
        inp.player_y,
        &fire_map,
    );
    fire_counts.is_best_bowl_filler = inp.is_best_bowl_filler;
    let Some(action) = crate::try_decide_handling_fire_from_rung(
        inp.profession_is_sticky,
        rung_label,
        inp.is_assigned_job,
        &sensors,
        fire_keeper,
        &fire_counts,
        fire_food_rt,
        fire_food_peers,
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
    let result = handling_fire_action_to_live_intent(tiles, inp, action);
    // Haxe: AiBase L1195–1206 shortCraftOnTarget(344) false → butt log / kindling same tick
    if !result.had_action {
        if let crate::HandlingFireAction::ShortCraftOnFire {
            actor,
            fire_object_id,
        } = action
        {
            if actor == crate::FIREWOOD && fire_object_id == crate::FIRE {
                let tail = crate::handling_fire::is_handling_fire_fire_fuel_tail(&sensors);
                return handling_fire_action_to_live_intent(tiles, inp, tail);
            }
        }
    }
    result
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
        BAKER_SCAN_RADIUS,
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
    counts.apply_popcorn_stock_from_map(
        inp.home_x,
        inp.home_y,
        inp.player_x,
        inp.player_y,
        &map,
    );
    counts.is_best_bowl_filler = inp.is_best_bowl_filler;
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

/// Roster for Haxe `getBestAiForObjByProfession('FIREKEEPER', obj)`.
///
/// `has_fire_keeper` is profession **weight > 0**, not lastProfession.
// Haxe: AiBase.getBestAiForObjByProfession ~1311 profession[name] > 0
pub fn fire_keeper_peers_from_state(
    state: &SimState,
    obj_x: i32,
    obj_y: i32,
    self_home_x: i32,
    self_home_y: i32,
) -> Vec<crate::FireKeeperPeer> {
    let following = &state.social.following;
    state
        .players
        .values()
        .map(|p| {
            let combat = state.combat.wound_of(p.p_id) > 0;
            let wounded =
                peer_roster_flags_for_player(p, &state.content, combat, Some(following)).is_wounded;
            let (ohx, ohy) = if p.home_x != 0 || p.home_y != 0 {
                (p.home_x, p.home_y)
            } else {
                (p.x, p.y)
            };
            let dx = (p.x - obj_x) as f32;
            let dy = (p.y - obj_y) as f32;
            crate::FireKeeperPeer {
                p_id: p.p_id,
                quad_dist_to_obj: dx * dx + dy * dy,
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                same_home: ohx == self_home_x && ohy == self_home_y,
                has_fire_keeper: p.fire_keeper_profession.weight > 0.0,
            }
        })
        .collect()
}

/// True when `self_p_id` wins Haxe FIREKEEPER distance pick vs `obj`.
// Haxe: getBestAiForObjByProfession('FIREKEEPER', obj); isHandlingFire ~1100 / ~1134
pub fn is_self_best_fire_keeper_from_state(
    state: &SimState,
    self_p_id: i32,
    obj_x: i32,
    obj_y: i32,
    self_home_x: i32,
    self_home_y: i32,
) -> bool {
    let min_age =
        if state.gameplay.min_age_to_eat.is_finite() && state.gameplay.min_age_to_eat >= 0.0 {
            state.gameplay.min_age_to_eat
        } else {
            MIN_AGE_TO_EAT
        };
    let peers = fire_keeper_peers_from_state(state, obj_x, obj_y, self_home_x, self_home_y);
    crate::is_self_best_fire_keeper_for_obj(self_p_id, &peers, min_age, MAX_AGE)
}

/// Home + firePlace best-AI flags for [`ProfessionScanInput`].
// Haxe: isHandlingFire ~1100 home; ~1134 firePlace (skipped when urgent)
pub fn best_fire_keeper_flags_from_state(
    state: &SimState,
    self_p_id: i32,
    home_x: i32,
    home_y: i32,
    fire_place_id: i32,
    fire_place_x: i32,
    fire_place_y: i32,
) -> (bool, bool) {
    let at_home = is_self_best_fire_keeper_from_state(
        state, self_p_id, home_x, home_y, home_x, home_y,
    );
    let (fx, fy) = if fire_place_id != 0 {
        (fire_place_x, fire_place_y)
    } else {
        (home_x, home_y)
    };
    let at_fire = is_self_best_fire_keeper_from_state(
        state, self_p_id, fx, fy, home_x, home_y,
    );
    (at_home, at_fire)
}

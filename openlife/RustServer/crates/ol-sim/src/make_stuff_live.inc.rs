// Haxe: AiBase.makeStuff live expand — bake + fire bodies (AI-MAKE-STUFF)
// Included from profession_scan.rs

/// Haxe late `makeStuff()` live expand (AI-SHEPHERD-MID + AI-MAKE-STUFF).
///
/// Sequential Haxe body: makeSharpieFood → doBaking(2) → doBasicFarming(2)
/// (mid sheep + after_sheep) → isSheepHerding(2) → makeFireFood(2).
// Haxe: AiBase.makeStuff ~4074 / doTimeStuffHelper ~835
pub fn make_stuff_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    farm_task: &mut FarmTaskState,
    farm_rt: &mut FarmProfessionRuntime,
    shepherd_rt: &mut ShepherdProfessionRuntime,
    baker_rt: &mut BakerProfessionRuntime,
    baker_task: &mut BakerTaskState,
    fire_rt: &mut crate::FireFoodProfessionRuntime,
) -> ProfessionScanTickResult {
    let farm_counts = farm_counts_from_scan(
        tiles,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        FARM_COUNT_RADIUS,
        inp.is_hungry,
        inp.basic_farmer_weight,
        inp.hardened_row_biome,
    );
    // 1) makeSharpieFood
    // Haxe: AiBase.makeStuff ~4077
    let sharpie = make_sharpie_food(&farm_counts);
    if sharpie.is_some() {
        return farm_action_to_live_intent(tiles, inp, sharpie, farm_rt);
    }
    // 2) doBaking(2) — full pure body (AI-MAKE-STUFF)
    // Haxe: AiBase.makeStuff ~4079
    {
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
            crate::MAKE_STUFF_FARM_MAX_PEOPLE,
            inp.peer_count,
            inp.was_idle,
            0,
        );
        if !matches!(action, BakeAction::None | BakeAction::Abort) {
            let r = bake_action_to_live_intent(tiles, inp, action);
            if r.had_action {
                return r;
            }
        }
    }
    // 3) doBasicFarming(2) including mid isSheepHerding(1) + after_sheep tail
    // Haxe: AiBase.makeStuff ~4080
    // Haxe: makeStuff → doBasicFarming(2)
    let farm = crate::do_basic_farming(
        &farm_counts,
        farm_task,
        true,
        crate::BASIC_FARM_DEFAULT_MAX_PROFESSION,
    );
    if farm.is_some() {
        let r = farm_action_to_live_intent(tiles, inp, farm, farm_rt);
        if r.had_action {
            return r;
        }
    }
    // 4) isSheepHerding(2)
    // Haxe: AiBase.makeStuff ~4081
    let sheep_counts = shepherd_counts_from_scan(
        tiles,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        true,
        inp.age,
        SHEPHERD_SHORTCRAFT_RADIUS,
    );
    let r = make_stuff_try_sheep(
        shepherd_rt,
        &sheep_counts,
        farm_task,
        inp.peer_count,
        inp.was_idle,
    );
    if r.action.is_some() {
        return shepherd_action_to_live_intent(tiles, inp, r.action);
    }
    // 5) makeFireFood(2) — full pure body (AI-MAKE-STUFF)
    // Haxe: AiBase.makeStuff ~4083
    {
        let map: Vec<crate::FireFoodMapObj> = tiles
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
            &map,
            crate::FIRE_FOOD_HOME_RADIUS,
            inp.is_hungry,
            false,
            inp.has_bean_seeds,
        );
        let has_corn = tiles.iter().any(|t| {
            matches!(
                t.parent_id,
                crate::BOWL_CORN_KERNELS
                    | crate::shepherd_profession::DRIED_EAR_OF_CORN
                    | crate::shepherd_profession::BOWL_CORN_COB
            )
        });
        fire_counts.has_corn_seeds = has_corn;
        let action = crate::make_fire_food(
            &fire_counts,
            fire_rt,
            crate::FIRE_FOOD_MAKE_STUFF_MAX_PEOPLE,
            inp.peer_count,
            inp.was_idle,
        );
        if action.is_some() {
            return fire_food_action_to_live_intent(tiles, inp, action);
        }
    }
    ProfessionScanTickResult::none()
}

/// Map a decided [`crate::FireFoodAction`] → live shortCraft intent (AI-MAKE-STUFF).
// Haxe: makeFireFood shortCraft / shortCraftOnGround / craftItem
pub fn fire_food_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: crate::FireFoodAction,
) -> ProfessionScanTickResult {
    use crate::FireFoodAction;
    match action {
        FireFoodAction::None | FireFoodAction::Abort => ProfessionScanTickResult::none(),
        FireFoodAction::CraftItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        FireFoodAction::ShortCraftOnGround { target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                crate::FIRE_FOOD_HOME_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    target,
                    inp.home_x,
                    inp.home_y,
                    crate::FIRE_FOOD_HOME_RADIUS,
                )
            });
            let forge =
                closest_forge_from_scan(tiles, inp.home_x, inp.home_y).map(|(_, x, y)| (x, y));
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                forge,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = crate::short_craft_on_ground_to_live_intent(inp.held_id, target, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
        FireFoodAction::ShortCraft { actor, target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                crate::FIRE_FOOD_HOME_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    target,
                    inp.home_x,
                    inp.home_y,
                    crate::FIRE_FOOD_HOME_RADIUS,
                )
            });
            let new_actor_count = tiles
                .iter()
                .filter(|t| t.parent_id == actor)
                .filter(|t| {
                    scan_chebyshev(inp.player_x, inp.player_y, t.x, t.y)
                        <= crate::FIRE_FOOD_HOME_RADIUS
                })
                .count() as i32;
            let apply = bake_action_short_craft_apply_ex(
                BakeAction::ShortCraft { actor, target },
                inp.held_id,
                new_actor_count,
                -1,
                inp.food_store,
                inp.transition_hungry_cost,
            )
            .unwrap_or(ShortCraftApply::Refuse);
            if matches!(apply, ShortCraftApply::RefuseHungry) {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::RefuseHungry,
                    had_action: true,
                };
            }
            if matches!(apply, ShortCraftApply::UseOnTarget { .. }) && target_tile.is_none() {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: target,
                        craft_if_needed: false,
                    },
                    had_action: true,
                };
            }
            let forge =
                closest_forge_from_scan(tiles, inp.home_x, inp.home_y).map(|(_, x, y)| (x, y));
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                forge,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}

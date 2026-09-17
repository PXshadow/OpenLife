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
    // 1) makeSharpieFood() default maxDistance=40 → getClosestObjectById quad from player
    // Haxe: AiBase.makeStuff ~4077; makeSharpieFood L4109
    let sharpie = make_sharpie_food_from_xy(
        inp.player_x,
        inp.player_y,
        inp.held_id,
        tiles.iter().map(|t| (t.parent_id, t.x, t.y)),
        MAKE_SHARPIE_FOOD_DEFAULT_MAX_DISTANCE,
    );
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
    // 3) doBasicFarming(2) including mid doWatering(3) + isSheepHerding(1) + after_sheep
    // Haxe: AiBase.makeStuff ~4080 / doBasicFarming ~2395
    let has_basic = has_or_become_profession(
        farm_rt,
        FarmProfession::BasicFarmer,
        crate::BASIC_FARM_DEFAULT_MAX_PROFESSION,
        farm_scan_peer_count(inp, FarmProfession::BasicFarmer),
        inp.was_idle,
    );
    let farm = crate::do_basic_farming_ex(
        &farm_counts,
        farm_task,
        has_basic,
        crate::BASIC_FARM_DEFAULT_MAX_PROFESSION,
        Some((
            farm_rt,
            farm_scan_peer_count(inp, FarmProfession::WaterBringer),
            inp.was_idle,
        )),
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
        fire_counts.apply_popcorn_stock_from_map(
            inp.home_x,
            inp.home_y,
            inp.player_x,
            inp.player_y,
            &map,
        );
        fire_counts.is_best_bowl_filler = inp.is_best_bowl_filler;
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
            // Haxe shortCraft(0, 1284, 20); other fire-food shortCraft default 20
            // from player, then home fallback at fire home r=30.
            // Haxe: AiBase.makeFireFood L4373
            let search_r = if target == crate::COOL_FLAT_ROCK {
                crate::COOL_FLAT_ROCK_SHORTCRAFT_DIST
            } else {
                crate::FIRE_FOOD_HOME_RADIUS
            };
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                search_r,
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
            let new_actor_count = short_craft_scan_new_actor_count(tiles, inp, actor, target);
            let apply = bake_action_short_craft_apply_ex(
                BakeAction::ShortCraft { actor, target },
                inp.held_id,
                new_actor_count,
                -1,
                inp.food_store,
                short_craft_pair_hungry_cost(inp, actor, target),
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

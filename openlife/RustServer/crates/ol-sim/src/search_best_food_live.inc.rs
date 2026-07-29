// Haxe: AiHelper.SearchBestFood live world scan — included from lib.rs (SEARCH-BEST-FOOD)
// Included after SimState is defined.

/// Haxe `AiHelper.SearchBestFood` — full scan (ground + containers + conservation).
///
/// Returns `(food_id, food_value, tx, ty, raw_quad_dist)` or None.
/// Prefer [`search_best_food_full`] when feeder/AI seed flags are needed.
// Haxe: AiHelper.SearchBestFood / SearchBestFoodHelperNew (SEARCH-BEST-FOOD)
pub fn search_best_food_nearby(
    state: &SimState,
    conn_id: u64,
    radius: i32,
) -> Option<(i32, i32, i32, i32, f32)> {
    let hit = search_best_food_full(state, conn_id, radius, None, None, false)?;
    Some((
        hit.food_id,
        hit.food_value,
        hit.tx,
        hit.ty,
        hit.raw_quad_dist,
    ))
}

/// Full Haxe `SearchBestFood(player, feedingPlayer, radius)`.
///
/// - `feeding_conn_id`: when feeding another, feeder conn (adds feeder distance).
/// - `ai_flags`: `Some` enables seed/danger AI gates; `None` = human / DisplayBestFood.
/// - `feed_other`: force canFeedToMeObj gates (also set when feeding_conn_id is Some).
// Haxe: AiHelper.SearchBestFoodHelperNew + processFood
pub fn search_best_food_full(
    state: &SimState,
    eater_conn_id: u64,
    radius: i32,
    feeding_conn_id: Option<u64>,
    ai_flags: Option<AiFoodSearchFlags>,
    feed_other: bool,
) -> Option<BestFoodHit> {
    let p = state.players.get(&eater_conn_id)?;
    if p.deleted {
        return None;
    }
    let (px, py) = (p.x, p.y);
    let food_store = p.food;
    let food_max = p.food_max;
    let craving = p.yum.currently_craving;
    // Haxe: canFeedToMeObj 837 + hasYellowFever
    let has_yellow_fever = crate::nested_body::is_yellow_fever(p.fever.as_ref());
    let count_eaten_fn = |food_id: i32| -> f32 {
        state
            .players
            .get(&eater_conn_id)
            .map(|pl| pl.yum.get_count_eaten(food_id))
            .unwrap_or(0.0)
    };

    let feed_other = feed_other || feeding_conn_id.is_some();
    let (feed_tx, feed_ty) = if let Some(fc) = feeding_conn_id {
        state
            .players
            .get(&fc)
            .map(|f| (Some(f.x), Some(f.y)))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    let deadly_tiles: Vec<(i32, i32)> = state
        .animals
        .animals
        .iter()
        .filter(|a| a.kind.is_deadly_for_ai())
        .map(|a| (a.x, a.y))
        .collect();

    // PATH-REACH: live AiPathReachMaps + blockedByAI for food skip / danger.
    // Haxe: AiHelper.SearchBestFood isObjectNotReachable / isObjectWithHostilePath
    let hostile_path_owned: Vec<(i32, i32)> = p.ai_path_reach.hostile_path_tiles().collect();
    let hostile_path: &[(i32, i32)] = &hostile_path_owned;
    // Snapshot keys so we do not hold `p` across later player re-borrows.
    let not_reachable_set: std::collections::HashSet<(i32, i32)> = p
        .ai_path_reach
        .not_reachable
        .keys()
        .copied()
        .chain(state.blocked_by_ai.keys().copied())
        .collect();

    let mut stock_tiles: Vec<StockTile> = Vec::new();
    let mut cands: Vec<SearchFoodCand> = Vec::new();
    let stock_r = radius.max(FOOD_STOCK_COUNT_RADIUS);
    // C-SS-FULL-TABLE: hot-reloaded FoodFactorEaten* for AI / DisplayBestFood scores
    // Haxe: WorldMap.getFoodFactor uses ServerSettings.FoodFactorEaten* statics
    let ff_bands = state.gameplay.food_factor_eaten_bands();

    let (map_w, map_h, wrap) = {
        let world = state.world.read().ok()?;
        let mw = world.width_tiles;
        let mh = world.height_tiles;
        let ww = world.wrap;

        // Haxe: half-open for (ty in baseY-radius...baseY+radius)
        for ty in (py - stock_r)..(py + stock_r) {
            for tx in (px - stock_r)..(px + stock_r) {
                let id = world.get_object(tx, ty);
                if id == 0 {
                    continue;
                }
                let base = state.content.resolve_base_id(id);
                let uses = world
                    .get_helper(tx, ty)
                    .map(|h| {
                        if h.uses_remaining > 0 {
                            h.uses_remaining
                        } else {
                            state
                                .content
                                .get(base)
                                .map(|d| if d.num_uses > 0 { d.num_uses } else { 1 })
                                .unwrap_or(1)
                        }
                    })
                    .unwrap_or(1);
                stock_tiles.push((tx, ty, base, uses.max(1)));
            }
        }

        for ty in (py - radius)..(py + radius) {
            for tx in (px - radius)..(px + radius) {
                let id = world.get_object(tx, ty);
                if id == 0 {
                    continue;
                }
                let base = state.content.resolve_base_id(id);
                let Some(def) = state.content.get(base) else {
                    continue;
                };

                let uses = world
                    .get_helper(tx, ty)
                    .map(|h| {
                        if h.uses_remaining > 0 {
                            h.uses_remaining
                        } else if def.num_uses > 0 {
                            def.num_uses
                        } else {
                            1
                        }
                    })
                    .unwrap_or_else(|| if def.num_uses > 0 { def.num_uses } else { 1 });

                let danger =
                    is_dangerous_near(tx, ty, FOOD_DANGER_RADIUS, &deadly_tiles, hostile_path);

                // PATH-REACH: personal notReachableObjects + blockedByAI.
                // Haxe: AiBase.isObjectNotReachable ~9273
                let not_reachable = not_reachable_set.contains(&(tx, ty));

                if def.num_slots > 0 && !container_blocks_remove(base) {
                    if let Some(h) = world.get_helper(tx, ty) {
                        for (i, &cid) in h.contained.iter().enumerate() {
                            if cid == 0 {
                                continue;
                            }
                            let cbase = state.content.resolve_base_id(cid);
                            let Some(cdef) = state.content.get(cbase) else {
                                continue;
                            };
                            // foodFromTarget content residual: only cdef.food_value
                            if cdef.food_value <= 0 {
                                continue;
                            }
                            let cuses = h
                                .slots
                                .get(i)
                                .map(|s| {
                                    if s.uses_remaining > 0 {
                                        s.uses_remaining
                                    } else if cdef.num_uses > 0 {
                                        cdef.num_uses
                                    } else {
                                        1
                                    }
                                })
                                .unwrap_or(1);
                            let food_id = cbase; // getFoodId simplified (no foodFromTarget table)
                            cands.push(SearchFoodCand {
                                parent_id: cbase,
                                food_id,
                                food_value: cdef.food_value,
                                tx,
                                ty,
                                count_eaten: count_eaten_fn(food_id),
                                number_of_uses: cuses,
                                index_in_container: i as i32,
                                is_dangerous: danger,
                                not_reachable,
                                // C-SS-FULL-TABLE: live FoodFactorEaten* bands (not default ModuleConst)
                                // Haxe: WorldMap.getFoodFactor + ServerSettings.FoodFactorEaten*
                                food_factor: state
                                    .world_food
                                    .get_food_factor_ex(food_id, &ff_bands),
                            });
                        }
                    }
                }

                // foodFromTarget content residual: only def.food_value > 0
                if def.food_value <= 0 {
                    continue;
                }
                let food_id = base;
                cands.push(SearchFoodCand {
                    parent_id: base,
                    food_id,
                    food_value: def.food_value,
                    tx,
                    ty,
                    count_eaten: count_eaten_fn(food_id),
                    number_of_uses: uses,
                    index_in_container: -1,
                    is_dangerous: danger,
                    not_reachable,
                    // C-SS-FULL-TABLE: live FoodFactorEaten* bands
                    // Haxe: WorldMap.getFoodFactor + ServerSettings.FoodFactorEaten*
                    food_factor: state.world_food.get_food_factor_ex(food_id, &ff_bands),
                });
            }
        }
        (mw, mh, ww)
    };

    let mut opts = ProcessFoodOpts::human(px, py, food_store, food_max, craving);
    opts.feed_other = feed_other;
    opts.feeding_tx = feed_tx;
    opts.feeding_ty = feed_ty;
    opts.ai = ai_flags;
    opts.map_w = map_w;
    opts.map_h = map_h;
    opts.wrap = wrap;
    opts.has_yellow_fever = has_yellow_fever;
    // YUM-LIVE-SETTINGS: SearchBestFood yum/meh band from live GameplayKnobs
    // Haxe: ServerSettings.YumBonus in processFood isYum
    opts.yum_bonus = state.gameplay.yum_bonus;

    let (i, score) = pick_best_search_food(&cands, &opts, &stock_tiles)?;
    Some(to_best_hit_ex(
        &cands[i],
        &score,
        px,
        py,
        map_w,
        map_h,
        wrap,
    ))
}

// AI-HANDLE-TEMP: live apply included from profession_scan.rs

/// Haxe `handleTemperature`: drink / GetOrCraft water / biome / fire / arrive / fail.
// Haxe: AiBase.handleTemperature ~1645; doTimeStuffHelper ~536 / ~586 / ~653
pub fn apply_handle_temperature_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    use crate::clothing_transitions::apply_drink_self_ex;
    use crate::fire_food_profession::LARGE_FAST_FIRE;
    use crate::handle_temperature::{
        count_little_kids, fail_warm_clear_place, get_close_biome, plan_handle_temperature,
        HandleTemperatureAction, HandleTemperatureInput, COOL_BIOMES, GET_CLOSE_BIOME_DIST,
        WARM_BIOMES,
    };
    use crate::player_soul::{
        is_super_cold_for_person_ex, is_super_hot_for_person_ex, person_looks_female,
    };
    use crate::short_craft_intent::{ShortCraftLiveApplyResult, ShortCraftLiveIntent};
    use ol_world::{DESERT, PASSABLE_RIVER};

    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let heat = p.heat;
    let last_heat = p.ai_last_heat;
    let ambient = p.last_temperature;
    let is_handling = p.ai_handling_temperature;
    let just_arrived = p.ai_temp_just_arrived;
    let held_raw = p.held_id;
    let held_id = if held_raw != 0 {
        state.content.resolve_base_id(held_raw)
    } else {
        0
    };
    let item_to_craft_id = p.craft_ai.item_to_craft_id;
    let age = p.age;
    let p_id = p.p_id;
    let display_id = p.display_object_id;
    let first = p.first_name.clone();
    let family = p.family_name.clone();
    let fire_id = p.ai_fire_place_id;
    let fire_x = p.ai_fire_place_x;
    let fire_y = p.ai_fire_place_y;
    let cold_place = p.cold_place;
    let warm_place = p.warm_place;
    let color = state.content.person_color(display_id);
    let is_super_hot = is_super_hot_for_person_ex(
        heat,
        color,
        state.gameplay.temperature_impact_below,
        state.gameplay.temperature_impact_color_factor,
    );
    let is_super_cold = is_super_cold_for_person_ex(
        heat,
        color,
        state.gameplay.temperature_impact_below,
        state.gameplay.temperature_impact_color_factor,
    );
    let male = state.content.get(display_id).map(|o| o.male);
    let female = person_looks_female(display_id, &first, "")
        || player_looks_female_for_clothing(&state.content, display_id);
    let female = if let Some(m) = male { !m } else { female };
    let _ = family;
    let is_winter = matches!(
        state.environment.season,
        crate::environment::Season::Winter
    );
    let min_age = if state.gameplay.min_age_to_eat.is_finite() {
        state.gameplay.min_age_to_eat
    } else {
        3.0
    };
    let kid_rows: Vec<(i32, f32, bool, Option<i32>, Option<i32>)> = state
        .players
        .values()
        .map(|o| {
            let node = state.social.lineages.get(&o.p_id);
            (
                o.p_id,
                o.age,
                o.deleted,
                node.and_then(|n| n.mother_id),
                node.and_then(|n| n.father_id),
            )
        })
        .collect();
    let little = count_little_kids(p_id, min_age, &kid_rows);
    let winter_female_with_kids = is_winter && female && little > 0;
    let fire_parent = if fire_id != 0 {
        state.content.resolve_base_id(fire_id)
    } else {
        0
    };
    let fire_heat_value = if fire_parent != 0 {
        state
            .content
            .get(fire_parent)
            .map(|o| o.heat_value)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let blocked = p.ai_path_reach.blocked_coords(Some(&state.blocked_by_ai));
    let (map_w, map_h, wrap, close_cool, close_warm) = {
        let w = state.world.read().unwrap();
        let mut cool_tiles: Vec<(i32, i32, u8)> = Vec::new();
        let mut warm_tiles: Vec<(i32, i32, u8)> = Vec::new();
        let start_x = px - GET_CLOSE_BIOME_DIST;
        let end_x = px + GET_CLOSE_BIOME_DIST;
        let start_y = py - GET_CLOSE_BIOME_DIST;
        let end_y = py + GET_CLOSE_BIOME_DIST;
        for ty in start_y..end_y {
            for tx in start_x..end_x {
                let b = w.get_biome(tx, ty);
                if b == 4 || b == PASSABLE_RIVER {
                    cool_tiles.push((tx, ty, b));
                } else if b == DESERT || b == 6 {
                    warm_tiles.push((tx, ty, b));
                }
            }
        }
        let cool = get_close_biome(
            px,
            py,
            &COOL_BIOMES,
            &cool_tiles,
            |x, y| blocked.contains(&(x, y)),
            w.width_tiles,
            w.height_tiles,
            w.wrap,
        );
        let warm = get_close_biome(
            px,
            py,
            &WARM_BIOMES,
            &warm_tiles,
            |x, y| blocked.contains(&(x, y)),
            w.width_tiles,
            w.height_tiles,
            w.wrap,
        );
        (w.width_tiles, w.height_tiles, w.wrap, cool, warm)
    };
    let _ = (map_w, map_h, wrap);

    let mut inp = HandleTemperatureInput {
        heat,
        last_heat,
        player_last_temperature: ambient,
        is_handling,
        just_arrived,
        is_super_hot,
        is_super_cold,
        winter_female_with_kids,
        held_id,
        item_to_craft_id,
        fire_x,
        fire_y,
        fire_parent,
        fire_heat_value,
        has_fire_place: fire_id != 0,
        close_cool,
        close_warm,
        cold_place,
        warm_place,
        px,
        py,
        age,
        skip_water: false,
    };
    let mut plan = plan_handle_temperature(inp);

    if matches!(plan.action, HandleTemperatureAction::GetOrCraftWater) {
        let r = apply_handle_temp_get_or_craft(state, outbound, conn_id, WATER_POUCH_THEN_BOWL);
        if !matches!(r, ShortCraftLiveApplyResult::Failed) {
            commit_handle_temp_plan(state, conn_id, plan);
            return r;
        }
        inp.skip_water = true;
        plan = plan_handle_temperature(inp);
    }

    commit_handle_temp_plan(state, conn_id, plan);
    match plan.action {
        HandleTemperatureAction::Idle => ShortCraftLiveApplyResult::Failed,
        HandleTemperatureAction::DrinkSelf => {
            let temp_reduction = state.gameplay.temperature_reduction_per_drinking;
            let max_stored = state.gameplay.max_stored_water;
            if let Some(p) = state.players.get_mut(&conn_id) {
                let _ = apply_drink_self_ex(p, &state.content, temp_reduction, max_stored);
            }
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
        }
        HandleTemperatureAction::CraftLargeFastFire => {
            apply_handle_temp_get_or_craft(state, outbound, conn_id, &[LARGE_FAST_FIRE])
        }
        HandleTemperatureAction::GetOrCraftWater => {
            apply_handle_temp_get_or_craft(state, outbound, conn_id, WATER_POUCH_THEN_BOWL)
        }
        HandleTemperatureAction::Relax => {
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
        }
        HandleTemperatureAction::Goto { x, y } => {
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y })
        }
        HandleTemperatureAction::KindlingOnFire { x, y } => {
            let kindling = apply_handle_temp_kindling(state, outbound, conn_id, x, y, held_id);
            if !matches!(kindling, ShortCraftLiveApplyResult::Failed) {
                return kindling;
            }
            let fire = apply_profession_scan_tick(
                state,
                outbound,
                conn_id,
                ProfessionScanKind::HandlingFire,
                "TEMPERATURE",
            );
            if !matches!(fire, ShortCraftLiveApplyResult::Failed) {
                return fire;
            }
            commit_handle_temp_plan(state, conn_id, fail_warm_clear_place(plan));
            ShortCraftLiveApplyResult::Failed
        }
        HandleTemperatureAction::HandlingFire => {
            let fire = apply_profession_scan_tick(
                state,
                outbound,
                conn_id,
                ProfessionScanKind::HandlingFire,
                "TEMPERATURE",
            );
            if !matches!(fire, ShortCraftLiveApplyResult::Failed) {
                return fire;
            }
            commit_handle_temp_plan(state, conn_id, fail_warm_clear_place(plan));
            ShortCraftLiveApplyResult::Failed
        }
    }
}

const WATER_POUCH_THEN_BOWL: &[i32] = &[
    crate::clothing_transitions::WATER_POUCH_ID,
    crate::clothing_transitions::WATER_BOWL_ID,
];

fn commit_handle_temp_plan(
    state: &mut crate::SimState,
    conn_id: u64,
    plan: crate::handle_temperature::HandleTemperaturePlan,
) {
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.ai_handling_temperature = plan.is_handling;
        p.ai_temp_just_arrived = plan.just_arrived;
        p.ai_last_heat = plan.last_heat;
        if plan.clear_cold_place {
            p.cold_place = None;
        }
        if plan.clear_warm_place {
            p.warm_place = None;
        }
    }
}

fn apply_handle_temp_kindling(
    state: &mut crate::SimState,
    outbound: &ol_net::OutboundHub,
    conn_id: u64,
    fx: i32,
    fy: i32,
    held_id: i32,
) -> crate::short_craft_intent::ShortCraftLiveApplyResult {
    use crate::handle_temperature::HANDLE_TEMP_KINDLING;
    use crate::short_craft_intent::{
        apply_short_craft_live_intent, ShortCraftLiveApplyResult, ShortCraftLiveIntent,
    };
    if held_id == HANDLE_TEMP_KINDLING {
        return apply_short_craft_live_intent(
            state,
            outbound,
            conn_id,
            ShortCraftLiveIntent::UseAt {
                x: fx,
                y: fy,
                target_id: 0,
                actor_id: HANDLE_TEMP_KINDLING,
            },
        );
    }
    let r = apply_handle_temp_get_or_craft(state, outbound, conn_id, &[HANDLE_TEMP_KINDLING]);
    if !matches!(r, ShortCraftLiveApplyResult::Failed) {
        r
    } else {
        ShortCraftLiveApplyResult::Failed
    }
}

fn apply_handle_temp_get_or_craft(
    state: &mut crate::SimState,
    outbound: &ol_net::OutboundHub,
    conn_id: u64,
    products: &[i32],
) -> crate::short_craft_intent::ShortCraftLiveApplyResult {
    use crate::get_or_craft::world_objs_from_ids;
    use crate::short_craft_intent::{
        apply_short_craft_live_intent, ShortCraftLiveApplyResult, ShortCraftLiveIntent,
    };
    use crate::{
        expand_craft_item_player_sticky_scan, CraftLiveExpandOpts, CraftScanFilters,
    };

    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let held_id = p.held_id;
    let home = if p.home_x != 0 || p.home_y != 0 {
        Some((p.home_x, p.home_y))
    } else {
        None
    };
    let is_smith = crate::resolve_smith_assigned_job(&p.smith_profession);
    let now_sec = state.sim_time as f64;
    let blocked = p.ai_path_reach.blocked_coords(Some(&state.blocked_by_ai));
    let radius = 40i32;
    let mut items: Vec<(i32, i32, i32)> = Vec::new();
    {
        let w = state.world.read().unwrap();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = px + dx;
                let y = py + dy;
                let id = w.get_object(x, y);
                if id != 0 {
                    items.push((id, x, y));
                }
            }
        }
    }
    let objs = world_objs_from_ids(&items, None);
    let graph = state.craft_graph.clone();
    let mut opts = CraftLiveExpandOpts {
        home,
        is_or_can_smith: is_smith,
        now_sec,
        water_source_ids: state.water_source_ids.clone(),
        bucket_water_source_ids: state.bucket_water_source_ids.clone(),
        ..Default::default()
    }
    .with_content_craft_gates(&state.content);
    state.gameplay.apply_craft_ai_search_knobs(&mut opts);
    let pile_id_for = |_id: i32| 0i32;
    let scan = CraftScanFilters::new().with_blocked(&blocked);
    for &product_id in products {
        let intent = {
            let Some(p) = state.players.get_mut(&conn_id) else {
                return ShortCraftLiveApplyResult::Failed;
            };
            expand_craft_item_player_sticky_scan(
                product_id,
                &objs,
                px,
                py,
                held_id,
                &pile_id_for,
                Some((px, py)),
                &graph,
                &opts,
                &mut p.craft_ai,
                scan,
            )
        };
        if matches!(intent, ShortCraftLiveIntent::None) {
            continue;
        }
        let r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
        if !matches!(r, ShortCraftLiveApplyResult::Failed) {
            return r;
        }
    }
    ShortCraftLiveApplyResult::Failed
}

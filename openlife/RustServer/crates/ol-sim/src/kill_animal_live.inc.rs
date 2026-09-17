// Haxe killAnimal L5878–5900: wolf-at-home look + HUNTER weight / hasOrBecome

use crate::kill_animal::{
    kill_animal_prefix, wolf_tile_allowed, KillAnimalPrefixInput, KILL_ANIMAL_WOLF_SEARCH,
    TIME_HELPER_TICK_TIME, WOLF,
};

/// Persist prefix sticky onto [`crate::Player`].
fn write_kill_animal_prefix_player(
    p: &mut crate::Player,
    animal_target: Option<(i32, i32, i32)>,
    time_looked_tick: f32,
    hunter: crate::HunterProfessionRuntime,
) {
    match animal_target {
        Some((id, x, y)) => {
            p.ai_animal_target_id = id;
            p.ai_animal_target_x = x;
            p.ai_animal_target_y = y;
        }
        None => {
            p.ai_animal_target_id = 0;
            p.ai_animal_target_x = 0;
            p.ai_animal_target_y = 0;
        }
    }
    p.ai_time_looked_for_deadly_animal_at_home = time_looked_tick;
    if hunter.is_last_hunter && !p.hunter_profession.is_last_hunter {
        p.last_profession = Some(crate::HUNTER_PROFESSION_KEY.into());
    }
    p.hunter_profession = hunter;
}

/// Haxe `killAnimal` L5878–5900 side effect (does not consume the think tick).
// Haxe: AiBase.killAnimal L5878–5900
pub fn apply_kill_animal_prefix_tick(
    state: &mut crate::SimState,
    conn_id: u64,
) -> crate::KillAnimalPrefixKind {
    let Some(p) = state.players.get(&conn_id) else {
        return crate::KillAnimalPrefixKind::Stop;
    };
    let px = p.x;
    let py = p.y;
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        px
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        py
    };
    let clothing = p.clothing_parent_ids();
    let animal_target = if p.ai_animal_target_id != 0 {
        Some((
            p.ai_animal_target_id,
            p.ai_animal_target_x,
            p.ai_animal_target_y,
        ))
    } else {
        None
    };
    let time_looked = p.ai_time_looked_for_deadly_animal_at_home;
    let mut hunter = p.hunter_profession.clone();
    let was_idle = 0.0;
    let now_tick = state.tick as f32;
    let deadly = state.animals_share.as_ref().and_then(|share| {
        share
            .read()
            .ok()?
            .get_close_deadly_animal(px, py, crate::DEADLY_ANIMAL_SEARCH_DIST)
            .map(|d| (d.id, d.x, d.y))
    });
    let tiles = {
        let w = state.world.read().unwrap();
        scan_world_radius(
            &w,
            Some(&state.content),
            home_x,
            home_y,
            KILL_ANIMAL_WOLF_SEARCH,
        )
    };
    let wolf_tiles: Vec<(i32, i32, i32)> = tiles
        .iter()
        .filter(|t| t.parent_id == WOLF)
        .filter(|t| wolf_tile_allowed(t.floor_id, t.is_food, t.is_permanent))
        .map(|t| (t.parent_id, t.x, t.y))
        .collect();
    let peer = peer_count_for_kind(
        ProfessionScanKind::Hunting,
        state,
        conn_id,
        home_x,
        home_y,
    );
    let inp = KillAnimalPrefixInput {
        animal: deadly,
        animal_target,
        time_looked_tick: time_looked,
        now_tick,
        tick_time: TIME_HELPER_TICK_TIME,
        clothing_ids: &clothing,
        home_tiles: &wolf_tiles,
        home_x,
        home_y,
        hunter_peer_count: peer,
        was_idle,
    };
    let result = kill_animal_prefix(&inp, &mut hunter);
    if let Some(p) = state.players.get_mut(&conn_id) {
        write_kill_animal_prefix_player(p, result.animal_target, result.time_looked_tick, hunter);
    }
    result.kind
}

/// KillAnimal rung: prefix + body L5902–5964.
// Haxe: AiBase.killAnimal L5878–5964
pub fn apply_kill_animal_tick(
    state: &mut crate::SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    if apply_kill_animal_prefix_tick(state, conn_id) == crate::KillAnimalPrefixKind::Stop {
        return ShortCraftLiveApplyResult::Failed;
    }
    apply_kill_animal_body_tick(state, outbound, conn_id)
}

fn kill_animal_bow_killable(content: &ol_content::ContentDb, id: i32) -> bool {
    content
        .find_transition(crate::attack_player::BOW_AND_ARROW, id)
        .is_some()
}

fn get_weapon_to_live(gw: crate::attack_player::GetWeaponAction, held_id: i32) -> ShortCraftLiveIntent {
    attack_player_action_to_live_intent(
        crate::attack_player::AttackPlayerAction::GetWeapon(gw),
        held_id,
    )
}

/// Haxe `killAnimal` L5902–5964 live apply.
// Haxe: AiBase.killAnimal L5902–5964
fn apply_kill_animal_body_tick(
    state: &mut crate::SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    use crate::attack_player::{
        deadly_distance_for_held, AttackPlayerClothing, AttackPlayerInput, BOW_AND_ARROW,
        GET_WEAPON_DROP_HOME, MIN_AI_AGE_FOR_COMBAT, WEAPON_SEARCH_DIST,
    };
    use crate::kill_animal::{
        kill_animal_body, kill_animal_bow_hunt, KillAnimalAction, KillAnimalBodyInput,
        KILL_ANIMAL_SNAKE_RADIUS,
    };
    use crate::hunting::{HUNT_KNIFE, RATTLE_SNAKE};
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        px
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        py
    };
    let food = p.food;
    let age = p.age;
    let held_id = p.held_id;
    let held_uses = p.held_uses;
    let moving = p.moving || p.move_path.is_some();
    let clothing_ids = p.clothing_parent_ids();
    let clothing_uses = p.clothing_uses_remaining();
    let clay = held_contains_clay_from_player(p);
    let (ex, ey) = p.exact_xy();
    let animal_target = if p.ai_animal_target_id != 0 {
        Some((
            p.ai_animal_target_id,
            p.ai_animal_target_x,
            p.ai_animal_target_y,
        ))
    } else {
        None
    };
    let parent = state
        .content
        .dummy_parent
        .get(&held_id)
        .copied()
        .unwrap_or(held_id);
    let held_name = state
        .content
        .get(held_id)
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let content_dd = state
        .content
        .get(held_id)
        .map(|d| d.deadly_distance)
        .unwrap_or(0.0);
    let bow_min = state
        .content
        .get(BOW_AND_ARROW)
        .map(|d| d.min_pickup_age as f32)
        .unwrap_or(0.0);
    let bow_parent = state
        .content
        .dummy_parent
        .get(&BOW_AND_ARROW)
        .copied()
        .unwrap_or(BOW_AND_ARROW);
    let bow_use = state
        .content
        .get(BOW_AND_ARROW)
        .map(|d| d.use_distance as f32)
        .unwrap_or(0.0);
    let tiles = {
        let w = state.world.read().unwrap();
        scan_world_radius(&w, Some(&state.content), px, py, WEAPON_SEARCH_DIST)
    };
    let nearby: Vec<(i32, i32, i32, bool)> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| (t.parent_id, t.x, t.y, t.is_permanent))
        .collect();
    let deadly = state
        .animals
        .get_close_deadly_animal(px, py, crate::DEADLY_ANIMAL_SEARCH_DIST)
        .map(|d| (d.kind.object_id(), d.x, d.y));
    let snake_tile = closest_by_parent_id(&tiles, RATTLE_SNAKE, px, py, crate::DEADLY_ANIMAL_SEARCH_DIST);
    let animal = deadly.or_else(|| snake_tile.map(|t| (t.parent_id, t.x, t.y)));
    let animal_killable = animal
        .map(|(id, _, _)| kill_animal_bow_killable(&state.content, id))
        .unwrap_or(false);
    let target_killable = animal_target
        .map(|(id, _, _)| kill_animal_bow_killable(&state.content, id))
        .unwrap_or(false);
    let weapon = AttackPlayerInput {
        target: None,
        food_store: food,
        self_wounded: p.is_wounded_held(is_wound_object(&state.content, held_id)),
        age,
        min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
        holding_weapon: crate::is_holding_weapon(held_id, &held_name),
        held_id,
        held_parent_id: parent,
        is_moving: moving,
        player_x: px,
        player_y: py,
        exact_x: ex,
        exact_y: ey,
        home_x,
        home_y,
        deadly_distance: deadly_distance_for_held(held_id, content_dd),
        clothing: AttackPlayerClothing::from_ids(&clothing_ids),
        weapon_tiles: &nearby,
    };
    let body_inp = KillAnimalBodyInput {
        food_store: food,
        age,
        bow_min_pickup_age: bow_min,
        animal,
        animal_target,
        animal_killable_by_bow: animal_killable,
        target_killable_by_bow: target_killable,
        player_x: px,
        player_y: py,
        held_parent_id: parent,
        bow_parent_id: bow_parent,
        bow_use_distance: bow_use,
        weapon,
    };
    let mut result = kill_animal_body(&body_inp);
    if matches!(result.action, KillAnimalAction::ShortCraftSnake) {
        let snake = closest_by_parent_id(&tiles, RATTLE_SNAKE, px, py, KILL_ANIMAL_SNAKE_RADIUS);
        let (target_uses, target_biome) = snake
            .map(|t| (t.uses, Some(t.biome)))
            .unwrap_or((1, None));
        let sc_inp = crate::farmer_profession::ShortCraftInput {
            held_id,
            actor_id: HUNT_KNIFE,
            target_id: RATTLE_SNAKE,
            target_uses,
            target_biome,
            has_carrot_seeds: true,
            new_actor_count: 0,
            max_new_actor: -1,
            try_weak_skewer_first: false,
            craft_actor_if_needed: true,
            food_store: food,
            transition_hungry_cost: crate::content_pair_hungry_work_cost(
                &state.content,
                HUNT_KNIFE,
                RATTLE_SNAKE,
                5.0,
            ),
        };
        let apply = crate::farmer_profession::short_craft_apply_resolved(sc_inp);
        if !matches!(
            apply,
            crate::farmer_profession::ShortCraftApply::Refuse
                | crate::farmer_profession::ShortCraftApply::RefuseHungry
        ) {
            let ctx = build_intent_ctx_ex(
                &tiles,
                px,
                py,
                home_x,
                home_y,
                snake,
                None,
                true,
                held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            if !matches!(intent, ShortCraftLiveIntent::None) {
                write_kill_animal_target(state, conn_id, result.animal_target);
                return apply_short_craft_live_intent(state, outbound, conn_id, intent);
            }
        }
        result = kill_animal_bow_hunt(&body_inp);
    }
    write_kill_animal_target(state, conn_id, result.animal_target);
    match result.action {
        KillAnimalAction::None => ShortCraftLiveApplyResult::Failed,
        KillAnimalAction::ShortCraftSnake => ShortCraftLiveApplyResult::Failed,
        KillAnimalAction::GetWeapon(gw) => {
            if matches!(gw, crate::attack_player::GetWeaponAction::DropHeld) {
                let intent = super::smart_drop_held_profession_ex_content(
                    &tiles,
                    held_id,
                    held_uses,
                    px,
                    py,
                    home_x,
                    home_y,
                    food,
                    false,
                    GET_WEAPON_DROP_HOME,
                    clay,
                    moving,
                    &clothing_ids,
                    &clothing_uses,
                    Some(&state.content),
                );
                return apply_short_craft_live_intent(state, outbound, conn_id, intent);
            }
            apply_short_craft_live_intent(state, outbound, conn_id, get_weapon_to_live(gw, held_id))
        }
        KillAnimalAction::Goto { x, y } => {
            let moving_now = state
                .players
                .get(&conn_id)
                .map(|pl| pl.moving || pl.move_path.is_some())
                .unwrap_or(moving);
            if moving_now {
                let same = state
                    .players
                    .get(&conn_id)
                    .map(|pl| pl.ai_last_goto_obj_x == x && pl.ai_last_goto_obj_y == y)
                    .unwrap_or(false);
                if same {
                    return ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait);
                }
            }
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.ai_last_goto_obj_x = x;
                pl.ai_last_goto_obj_y = y;
                pl.ai_did_not_reach_animal_target = 0;
            }
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y })
        }
        KillAnimalAction::Use { x, y } => {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.ai_did_not_reach_food = 0.0;
            }
            let intent = ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: result.animal_target.map(|(id, _, _)| id).unwrap_or(0),
                actor_id: held_id,
            };
            let r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
            if matches!(r, ShortCraftLiveApplyResult::Failed) {
                ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
            } else {
                r
            }
        }
    }
}

fn write_kill_animal_target(
    state: &mut crate::SimState,
    conn_id: u64,
    animal_target: Option<(i32, i32, i32)>,
) {
    if let Some(p) = state.players.get_mut(&conn_id) {
        match animal_target {
            Some((id, x, y)) => {
                p.ai_animal_target_id = id;
                p.ai_animal_target_x = x;
                p.ai_animal_target_y = y;
            }
            None => {
                p.ai_animal_target_id = 0;
                p.ai_animal_target_x = 0;
                p.ai_animal_target_y = 0;
            }
        }
    }
}

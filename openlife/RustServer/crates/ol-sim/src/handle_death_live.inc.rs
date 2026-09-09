// AI-HANDLE-DEATH: live apply included from profession_scan.rs

/// Haxe `handleDeath`: wipe jobs → GRAVEKEEPER, graves if near home, else go home, else drop.
// Haxe: AiBase.handleDeath ~1611; doTimeStuffHelper ~554
pub fn apply_handle_death_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    use crate::handle_death::{
        handle_death_after_graves_miss, handle_death_last_profession, plan_handle_death,
        wipe_jobs_assign_grave_keeper, HandleDeathAction, HANDLE_DEATH_GO_HOME_TILES,
        HANDLE_DEATH_TIME_BUMP,
    };
    use crate::ai_say_helper::{go_home_goal_xy, go_home_move_target};
    use crate::short_craft_intent::{
        apply_short_craft_live_intent, ShortCraftLiveApplyResult, ShortCraftLiveIntent,
    };

    if let Some(adv) = advance_player_use_held_staging(state, conn_id) {
        match adv {
            crate::short_craft_intent::UseHeldAdvance::UseNow { x, y } => {
                if let Some(p) = state.players.get_mut(&conn_id) {
                    p.craft_ai.use_held = None;
                }
                return apply_short_craft_live_intent(
                    state,
                    outbound,
                    conn_id,
                    ShortCraftLiveIntent::UseAt {
                        x,
                        y,
                        target_id: 0,
                        actor_id: 0,
                    },
                );
            }
            crate::short_craft_intent::UseHeldAdvance::Goto { x, y } => {
                return ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y });
            }
            crate::short_craft_intent::UseHeldAdvance::Wait => {
                return ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait);
            }
            crate::short_craft_intent::UseHeldAdvance::DropHeld => {
                let (px, py) = state
                    .players
                    .get(&conn_id)
                    .map(|p| (p.x, p.y))
                    .unwrap_or((0, 0));
                return apply_short_craft_live_intent(
                    state,
                    outbound,
                    conn_id,
                    ShortCraftLiveIntent::DropAt { x: px, y: py },
                );
            }
            crate::short_craft_intent::UseHeldAdvance::Cancel => {
                if let Some(p) = state.players.get_mut(&conn_id) {
                    p.craft_ai.use_held = None;
                }
            }
        }
    }

    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let max_age = if state.gameplay.max_age.is_finite() && state.gameplay.max_age > 2.0 {
        state.gameplay.max_age
    } else {
        crate::HANDLE_DEATH_MAX_AGE_DEFAULT
    };
    let age = p.age;
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
    let fire = if p.ai_fire_place_id != 0 {
        Some((p.ai_fire_place_x, p.ai_fire_place_y))
    } else {
        None
    };
    let (mtx, mty) = go_home_move_target(home_x, home_y, fire);
    let home_quad = (px - home_x) * (px - home_x) + (py - home_y) * (py - home_y);
    let move_quad = (px - mtx) * (px - mtx) + (py - mty) * (py - mty);
    let is_using = p.craft_ai.use_held.is_some();
    let is_removing = p.craft_ai.remove_from_container.is_some();
    let is_moving = p.moving || p.move_path.is_some();
    let held_id = p.held_id;
    let p_id = p.p_id;
    let seed = (p_id as u32).wrapping_mul(1103515245).wrapping_add(px as u32);
    let rand = (seed as f32) / (u32::MAX as f32);

    let plan = plan_handle_death(
        age,
        max_age,
        is_removing,
        is_using,
        is_moving,
        home_quad,
        move_quad,
        rand,
    );
    if matches!(plan.action, HandleDeathAction::NotOld) {
        return ShortCraftLiveApplyResult::Failed;
    }

    if let Some(p) = state.players.get_mut(&conn_id) {
        wipe_jobs_assign_grave_keeper(
            &mut p.farm_profession,
            &mut p.smith_profession,
            &mut p.baker_profession,
            &mut p.shepherd_profession,
            &mut p.pottery_profession,
            &mut p.fire_food_profession,
            &mut p.fire_keeper_profession,
            &mut p.grave_keeper_profession,
            &mut p.hunter_profession,
            &mut p.lumberjack_profession,
            &mut p.collector_profession,
            &mut p.foodserver_profession,
        );
        p.last_profession = Some(handle_death_last_profession().into());
        let _ = plan.say;
    }

    let mut action = plan.action;
    if action == HandleDeathAction::Busy {
        if is_removing {
            if let Some(r) = crate::short_craft_intent::apply_player_remove_from_container_tick(
                state, outbound, conn_id,
            ) {
                return r;
            }
        }
        return ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait);
    }
    if action == HandleDeathAction::GravesThenRest {
        let r = apply_profession_scan_tick(
            state,
            outbound,
            conn_id,
            ProfessionScanKind::HandlingGraves,
            "HANDLE_DEATH",
        );
        if !matches!(r, ShortCraftLiveApplyResult::Failed) {
            return r;
        }
        action = handle_death_after_graves_miss(move_quad);
    }
    match action {
        HandleDeathAction::GoHome => {
            let (gx, gy) = go_home_goal_xy(mtx, mty, seed);
            let _ = HANDLE_DEATH_GO_HOME_TILES;
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x: gx, y: gy })
        }
        HandleDeathAction::DropHeld => {
            let _ = HANDLE_DEATH_TIME_BUMP;
            if held_id != 0 {
                apply_short_craft_live_intent(
                    state,
                    outbound,
                    conn_id,
                    ShortCraftLiveIntent::DropAt { x: px, y: py },
                )
            } else {
                ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
            }
        }
        HandleDeathAction::Busy | HandleDeathAction::NotOld | HandleDeathAction::GravesThenRest => {
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
        }
    }
}

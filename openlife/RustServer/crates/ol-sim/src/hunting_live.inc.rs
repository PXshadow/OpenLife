// AI-JOB-HUNT: live scan helpers included from profession_scan.rs

/// Hunter / isHunting: sensors → pure body → shortCraft live intent.
// Haxe: isHunting assigned|last ~746; mid age>14 ~655
pub fn hunting_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    hunter: &mut crate::HunterProfessionRuntime,
) -> ProfessionScanTickResult {
    use crate::hunting::{
        hunting_sensors_from_closest, try_decide_hunting_from_rung, HuntingAction, FIREBRAND,
        HUNTING_SHORTCRAFT_RADIUS, HUNT_KNIFE, MOSQUITO_SWARM, MOSQUITO_SWARM_JUST_BIT,
        RATTLE_SNAKE,
    };
    let snake = closest_by_parent_id(
        tiles,
        RATTLE_SNAKE,
        inp.player_x,
        inp.player_y,
        HUNTING_SHORTCRAFT_RADIUS,
    )
    .map(|t| {
        let d = (t.x - inp.player_x).abs().max((t.y - inp.player_y).abs());
        (d, t.x, t.y)
    });
    let mosquito_bit = closest_by_parent_id(
        tiles,
        MOSQUITO_SWARM_JUST_BIT,
        inp.player_x,
        inp.player_y,
        HUNTING_SHORTCRAFT_RADIUS,
    )
    .map(|t| {
        let d = (t.x - inp.player_x).abs().max((t.y - inp.player_y).abs());
        (d, t.x, t.y)
    });
    let mosquito = closest_by_parent_id(
        tiles,
        MOSQUITO_SWARM,
        inp.player_x,
        inp.player_y,
        HUNTING_SHORTCRAFT_RADIUS,
    )
    .map(|t| {
        let d = (t.x - inp.player_x).abs().max((t.y - inp.player_y).abs());
        (d, t.x, t.y)
    });
    let sensors = hunting_sensors_from_closest(
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.age,
        snake,
        mosquito_bit,
        mosquito,
    );
    let was_idle = if inp.profession_is_sticky { 0.0 } else { inp.was_idle };
    let Some(action) = try_decide_hunting_from_rung(
        rung_label,
        inp.is_assigned_job,
        &sensors,
        hunter,
        inp.peer_count,
        was_idle,
    ) else {
        return ProfessionScanTickResult::none();
    };
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    match action {
        HuntingAction::None => ProfessionScanTickResult::none(),
        HuntingAction::ShortCraft { actor, target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                HUNTING_SHORTCRAFT_RADIUS,
            );
            let (target_uses, target_biome) = target_tile
                .map(|t| (t.uses, Some(t.biome)))
                .unwrap_or((1, None));
            let sc_inp = crate::farmer_profession::ShortCraftInput {
                held_id: inp.held_id,
                actor_id: actor,
                target_id: target,
                target_uses,
                target_biome,
                has_carrot_seeds: true,
                new_actor_count: 0,
                max_new_actor: -1,
                try_weak_skewer_first: false,
                craft_actor_if_needed: true,
                food_store: inp.food_store,
                transition_hungry_cost: short_craft_pair_hungry_cost(inp, actor, target),
            };
            let apply = crate::farmer_profession::short_craft_apply_resolved(sc_inp);
            if matches!(
                apply,
                crate::farmer_profession::ShortCraftApply::UseOnTarget { .. }
            ) && target_tile.is_none()
            {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: target,
                        craft_if_needed: false,
                    },
                    had_action: true,
                };
            }
            if matches!(apply, crate::farmer_profession::ShortCraftApply::DropHeld) {
                let intent = super::smart_drop_held_profession_ex_content(
                    tiles,
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
                    inp.content.as_deref(),
                );
                return ProfessionScanTickResult {
                    had_action: super::drop_held_live_intent_actionable(intent)
                        || !matches!(intent, ShortCraftLiveIntent::None),
                    intent,
                };
            }
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                None,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            let _ = (HUNT_KNIFE, FIREBRAND);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}

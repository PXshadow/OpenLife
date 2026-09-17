// AI-JOB-COLLECT: live scan helpers included from profession_scan.rs

/// Collector / isCollecting: sensors → makeOrCollect / shortCraft / nested smith.
// Haxe: isCollecting assigned|last ~718; after critical isCollecting(1) ~760
pub fn collecting_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    collector: &mut crate::CollectorProfessionRuntime,
    smith_rt: &mut SmithProfessionRuntime,
) -> ProfessionScanTickResult {
    use crate::collecting::{
        collecting_sensors, try_decide_collecting_from_rung, CollectingAction,
    };
    use crate::get_or_craft::{CraftAndDropApply, CraftWorldObj};
    let sensors = collecting_sensors(
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        inp.age,
    );
    let objs: Vec<CraftWorldObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| CraftWorldObj::simple(t.parent_id, t.x, t.y))
        .collect();
    let was_idle = if inp.profession_is_sticky {
        0.0
    } else {
        inp.was_idle
    };
    let Some(action) = try_decide_collecting_from_rung(
        rung_label,
        inp.is_assigned_job,
        &sensors,
        collector,
        inp.peer_count,
        was_idle,
        &objs,
    ) else {
        return ProfessionScanTickResult::none();
    };
    match action {
        CollectingAction::None => ProfessionScanTickResult::none(),
        CollectingAction::CraftAndDrop { which_id, apply } => {
            collecting_craft_drop_to_live(tiles, inp, which_id, apply)
        }
        CollectingAction::ShortCraft {
            actor,
            target,
            radius,
            max_new_actor,
            craft_actor_if_needed,
        } => collecting_short_craft_to_live(
            tiles,
            inp,
            actor,
            target,
            radius,
            max_new_actor,
            craft_actor_if_needed,
        ),
        CollectingAction::DeferSmithing => {
            let mut smith_inp = inp.clone();
            smith_inp.is_assigned_job = false;
            smith_inp.profession_is_sticky = false;
            let smith = smith_profession_scan_tick(tiles, &smith_inp, "AGE_ROTATED_JOB", smith_rt);
            if smith.had_action {
                return smith;
            }
            // Haxe: doSmithing(1) false → continue rabbits / mutton / pork
            match crate::collecting::collecting_after_smith(&sensors, collector, &objs) {
                CollectingAction::None | CollectingAction::DeferSmithing => {
                    ProfessionScanTickResult::none()
                }
                CollectingAction::CraftAndDrop { which_id, apply } => {
                    collecting_craft_drop_to_live(tiles, inp, which_id, apply)
                }
                CollectingAction::ShortCraft {
                    actor,
                    target,
                    radius,
                    max_new_actor,
                    craft_actor_if_needed,
                } => collecting_short_craft_to_live(
                    tiles,
                    inp,
                    actor,
                    target,
                    radius,
                    max_new_actor,
                    craft_actor_if_needed,
                ),
            }
        }
    }
}

fn collecting_craft_drop_to_live(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    which_id: i32,
    apply: crate::get_or_craft::CraftAndDropApply,
) -> ProfessionScanTickResult {
    use crate::get_or_craft::CraftAndDropApply;
    let intent = match apply {
        CraftAndDropApply::AlreadyEnough => ShortCraftLiveIntent::None,
        CraftAndDropApply::GotoDropTarget { target_x, target_y } => {
            ShortCraftLiveIntent::Goto {
                x: target_x,
                y: target_y,
            }
        }
        CraftAndDropApply::DropNearTarget { target_x, target_y } => {
            let empty = tiles.iter().find(|t| {
                t.parent_id == 0 && (t.x - target_x).abs().max((t.y - target_y).abs()) <= 1
            });
            if let Some(e) = empty {
                ShortCraftLiveIntent::DropAt { x: e.x, y: e.y }
            } else {
                ShortCraftLiveIntent::DropAt {
                    x: target_x,
                    y: target_y,
                }
            }
        }
        CraftAndDropApply::Pickup { object_id, x, y } => ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id: object_id,
            actor_id: 0,
        },
        CraftAndDropApply::CraftItem { object_id } => ShortCraftLiveIntent::CraftItem { object_id },
    };
    let _ = (inp, which_id);
    if matches!(intent, ShortCraftLiveIntent::None) {
        ProfessionScanTickResult::none()
    } else {
        ProfessionScanTickResult {
            intent,
            had_action: true,
        }
    }
}

fn collecting_short_craft_to_live(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    actor: i32,
    target: i32,
    radius: i32,
    max_new_actor: i32,
    craft_actor_if_needed: bool,
) -> ProfessionScanTickResult {
    let target_tile = closest_by_parent_id(tiles, target, inp.player_x, inp.player_y, radius)
        .or_else(|| closest_by_parent_id(tiles, target, inp.home_x, inp.home_y, radius));
    let (target_uses, target_biome) = target_tile
        .map(|t| (t.uses, Some(t.biome)))
        .unwrap_or((1, None));
    let new_actor_count = short_craft_scan_new_actor_count(tiles, inp, actor, target);
    let sc_inp = crate::farmer_profession::ShortCraftInput {
        held_id: inp.held_id,
        actor_id: actor,
        target_id: target,
        target_uses,
        target_biome,
        has_carrot_seeds: true,
        new_actor_count,
        max_new_actor,
        try_weak_skewer_first: false,
        craft_actor_if_needed,
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
    if matches!(intent, ShortCraftLiveIntent::None) {
        ProfessionScanTickResult::none()
    } else {
        ProfessionScanTickResult {
            intent,
            had_action: true,
        }
    }
}



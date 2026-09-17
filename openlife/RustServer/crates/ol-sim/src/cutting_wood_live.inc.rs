// AI-JOB-LUMBER: live scan helpers included from profession_scan.rs

/// Lumberjack / isCuttingWood: sensors → GetCraftAndDrop → live intent.
// Haxe: isCuttingWood assigned|last ~733; low-priority ~808
pub fn cutting_wood_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    lumber: &mut crate::LumberjackProfessionRuntime,
) -> ProfessionScanTickResult {
    use crate::cutting_wood::{
        cutting_wood_sensors, try_decide_cutting_wood_from_rung, CuttingWoodAction,
    };
    use crate::get_or_craft::{CraftAndDropApply, CraftWorldObj};
    let sensors = cutting_wood_sensors(
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        inp.age,
        inp.fire_place_id,
        inp.fire_place_x,
        inp.fire_place_y,
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
    let Some(action) = try_decide_cutting_wood_from_rung(
        rung_label,
        inp.is_assigned_job,
        &sensors,
        lumber,
        inp.peer_count,
        was_idle,
        &objs,
    ) else {
        return ProfessionScanTickResult::none();
    };
    match action {
        CuttingWoodAction::None => ProfessionScanTickResult::none(),
        CuttingWoodAction::CraftAndDrop { which_id, apply } => {
            cutting_wood_apply_to_live_intent(tiles, inp, which_id, apply)
        }
        CuttingWoodAction::TryCleanup => expand_cleanup_live(tiles, inp),
    }
}

fn cutting_wood_apply_to_live_intent(
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
                t.parent_id == 0
                    && (t.x - target_x).abs().max((t.y - target_y).abs()) <= 1
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

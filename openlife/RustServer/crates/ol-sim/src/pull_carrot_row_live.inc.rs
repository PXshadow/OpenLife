// PULL-CARROT-ROW: live scan helpers included from profession_scan.rs

use crate::farmer_profession::{
    pull_carrot_row_if_needed, PullCarrotRowAction, PullCarrotRowInput, PULL_CARROT_ROW_SEARCH_DIST,
};

/// Mid `shortCraft(0, 400, 10)` — empty-hand USE on closest carrot row r=10.
// Haxe: AiBase.doTimeStuffHelper ~609
pub fn pull_carrot_row_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> ProfessionScanTickResult {
    let row = closest_by_parent_id(
        tiles,
        CARROT_ROW,
        inp.player_x,
        inp.player_y,
        PULL_CARROT_ROW_SEARCH_DIST,
    );
    let sensors = PullCarrotRowInput {
        held_id: inp.held_id,
        food_store: inp.food_store,
        transition_hungry_cost: short_craft_pair_hungry_cost(inp, 0, CARROT_ROW),
        has_carrot_seeds: inp.has_carrot_seeds,
        row: row.map(|t| (t.x, t.y, t.uses)),
    };
    match pull_carrot_row_if_needed(&sensors) {
        PullCarrotRowAction::None => ProfessionScanTickResult::none(),
        PullCarrotRowAction::UseEmptyOnRow { x, y } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: CARROT_ROW,
                actor_id: 0,
            },
            had_action: true,
        },
        PullCarrotRowAction::DropHeld => {
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
            ProfessionScanTickResult {
                had_action: super::drop_held_live_intent_actionable(intent)
                    || !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}

// FILL-BERRY-HELD: live scan helpers included from profession_scan.rs

use crate::baker_profession::{
    fill_berry_bowl_held_if_needed, FillBerryBowlHeldAction, FillBerryBowlHeldInput,
    FILL_BERRY_HELD_SEARCH_DIST,
};

fn closest_berry_bush_from_scan(
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
    max_r: i32,
) -> Option<ScanTile> {
    let wild = closest_by_parent_id(tiles, WILD_BUSH, player_x, player_y, max_r);
    let domestic = closest_by_parent_id(tiles, DOMESTIC_BUSH, player_x, player_y, max_r);
    match (wild, domestic) {
        (None, None) => None,
        (Some(t), None) | (None, Some(t)) => Some(t),
        (Some(a), Some(b)) => {
            let da = scan_chebyshev(player_x, player_y, a.x, a.y);
            let db = scan_chebyshev(player_x, player_y, b.x, b.y);
            if da < db || (da == db && (a.y < b.y || (a.y == b.y && a.x < b.x))) {
                Some(a)
            } else {
                Some(b)
            }
        }
    }
}

fn held_num_uses_from_scan_berry(inp: &ProfessionScanInput) -> i32 {
    inp.content
        .as_ref()
        .and_then(|c| c.get(inp.held_id))
        .map(|d| d.num_uses)
        .unwrap_or(0)
}

/// Mid `fillBerryBowlIfNeeded(true)` onlyFillHeld.
// Haxe: AiBase.doTimeStuffHelper ~627
pub fn fill_berry_bowl_held_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> ProfessionScanTickResult {
    let bush = closest_berry_bush_from_scan(
        tiles,
        inp.player_x,
        inp.player_y,
        FILL_BERRY_HELD_SEARCH_DIST,
    );
    let sensors = FillBerryBowlHeldInput {
        held_id: inp.held_id,
        held_uses: if inp.held_uses > 0 { inp.held_uses } else { 1 },
        held_num_uses: held_num_uses_from_scan_berry(inp),
        bush: bush.map(|t| (t.x, t.y, t.parent_id)),
    };
    match fill_berry_bowl_held_if_needed(&sensors) {
        FillBerryBowlHeldAction::None => ProfessionScanTickResult::none(),
        FillBerryBowlHeldAction::UseHeldOnBush { x, y, bush_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: bush_id,
                actor_id: BOWL_GOOSEBERRIES,
            },
            had_action: true,
        },
    }
}

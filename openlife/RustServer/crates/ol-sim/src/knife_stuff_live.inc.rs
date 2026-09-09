// AI-KNIFE-STUFF: live scan helpers included from profession_scan.rs

use crate::knife_stuff::{
    do_knife_stuff_ex, KnifeStuffAction, KNIFE, KNIFE_STUFF_DIST,
};

/// True when mid `doKnifeStuff` would USE (held Knife 560 + target r=20).
// Haxe: AiBase.doKnifeStuff ~876
pub fn knife_stuff_mid_pending(
    held_id: i32,
    player_x: i32,
    player_y: i32,
    tiles: &[ScanTile],
) -> bool {
    knife_stuff_profession_scan_tick_at(tiles, held_id, player_x, player_y, 20.0, 0.0).had_action
}

fn knife_stuff_profession_scan_tick_at(
    tiles: &[ScanTile],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    food_store: f32,
    hungry_cost: f32,
) -> ProfessionScanTickResult {
    let nearby: Vec<(i32, i32, i32)> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .filter(|t| scan_chebyshev(player_x, player_y, t.x, t.y) <= KNIFE_STUFF_DIST)
        .map(|t| (t.parent_id, t.x, t.y))
        .collect();
    match do_knife_stuff_ex(
        held_id,
        &nearby,
        player_x,
        player_y,
        food_store,
        hungry_cost,
    ) {
        KnifeStuffAction::None => ProfessionScanTickResult::none(),
        KnifeStuffAction::UseOnTarget { x, y, target_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id,
                actor_id: KNIFE,
            },
            had_action: true,
        },
    }
}

/// Mid held-knife shortCraft (bread / dough / wolf / bear, r=20, no craftActor).
// Haxe: AiBase.doKnifeStuff ~876; doTimeStuffHelper ~635; isConsideringMakingFood ~8549
pub fn knife_stuff_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> ProfessionScanTickResult {
    if !inp.target_reachable {
        return ProfessionScanTickResult::none();
    }
    let hungry = if inp.content.is_some() {
        // Per-pair cost is applied after the first matching target is chosen.
        0.0
    } else {
        inp.transition_hungry_cost
    };
    let r = knife_stuff_profession_scan_tick_at(
        tiles,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.food_store,
        hungry,
    );
    match r.intent {
        ShortCraftLiveIntent::UseAt { target_id, .. } if inp.content.is_some() => {
            let cost = short_craft_pair_hungry_cost(inp, KNIFE, target_id);
            knife_stuff_profession_scan_tick_at(
                tiles,
                inp.held_id,
                inp.player_x,
                inp.player_y,
                inp.food_store,
                cost,
            )
        }
        _ => r,
    }
}


// FILL-BEAN-BOWL: live scan helpers included from profession_scan.rs

use crate::farmer_profession::{
    bean_bowl_ids, fill_bean_bowl_if_needed, FillBeanBowlAction, FillBeanBowlInput,
    FILL_BEAN_BOWL_SEARCH_DIST,
};

/// Count dry-bean stock for the green-fill gate (tiles + held).
// Haxe: countCurrentObjects([1176, 1172])
fn count_dry_beans_from_scan(tiles: &[ScanTile], held_id: i32) -> i32 {
    let mut n = tiles
        .iter()
        .filter(|t| t.parent_id == BOWL_OF_DRY_BEANS || t.parent_id == DRY_BEAN_PLANTS)
        .count() as i32;
    if held_id == BOWL_OF_DRY_BEANS || held_id == DRY_BEAN_PLANTS {
        n += 1;
    }
    n
}

fn held_num_uses_from_scan(inp: &ProfessionScanInput) -> i32 {
    inp.content
        .as_ref()
        .and_then(|c| c.get(inp.held_id))
        .map(|d| d.num_uses)
        .unwrap_or(0)
}

/// Build Haxe `fillBeanBowlIfNeeded` sensors from profession scan tiles.
// Haxe: GetClosestObjectById plant/bowl r=40
pub fn fill_bean_bowl_input_from_scan(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    green_beans: bool,
    only_fill_held: bool,
) -> FillBeanBowlInput {
    let (bowl_id, plant_id) = bean_bowl_ids(green_beans);
    let plant = closest_by_parent_id(
        tiles,
        plant_id,
        inp.player_x,
        inp.player_y,
        FILL_BEAN_BOWL_SEARCH_DIST,
    );
    let bowl = closest_by_parent_id(
        tiles,
        bowl_id,
        inp.player_x,
        inp.player_y,
        FILL_BEAN_BOWL_SEARCH_DIST,
    );
    FillBeanBowlInput {
        held_id: inp.held_id,
        held_uses: if inp.held_uses > 0 { inp.held_uses } else { 1 },
        held_num_uses: held_num_uses_from_scan(inp),
        green_beans,
        only_fill_held,
        count_dry_beans: count_dry_beans_from_scan(tiles, inp.held_id),
        plant_xy: plant.map(|t| (t.x, t.y)),
        bowl_xy: bowl.map(|t| (t.x, t.y)),
        bowl_uses: bowl.map(|t| if t.uses > 0 { t.uses } else { 1 }).unwrap_or(1),
        bowl_num_uses: bowl.map(|t| t.num_uses).unwrap_or(0),
        is_best_bowl_filler: inp.is_best_bowl_filler,
    }
}

fn fill_bean_bowl_action_to_live(action: FillBeanBowlAction, held_id: i32) -> ProfessionScanTickResult {
    match action {
        FillBeanBowlAction::None => ProfessionScanTickResult::none(),
        FillBeanBowlAction::UseHeldOnPlant { x, y, plant_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: plant_id,
                actor_id: held_id,
            },
            had_action: true,
        },
        FillBeanBowlAction::PickupBowl { x, y, bowl_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: bowl_id,
                actor_id: 0,
            },
            had_action: true,
        },
        FillBeanBowlAction::GetClayBowl => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekOrCraft {
                actor: CLAY_BOWL,
                craft_if_needed: false,
            },
            had_action: true,
        },
    }
}

/// Low `fillBeanBowlIfNeeded()` (green beans, not onlyFillHeld).
// Haxe: AiBase.doTimeStuffHelper ~786
pub fn fill_bean_bowl_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> ProfessionScanTickResult {
    let sensors = fill_bean_bowl_input_from_scan(tiles, inp, true, false);
    fill_bean_bowl_action_to_live(fill_bean_bowl_if_needed(&sensors), inp.held_id)
}

/// Mid `fillBeanBowlIfNeeded(true, true)` then `(false, true)` — held path only.
// Haxe: AiBase.doTimeStuffHelper ~628–629
pub fn fill_bean_bowl_held_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> ProfessionScanTickResult {
    use crate::farmer_profession::fill_bean_bowl_held_if_needed;
    let green = fill_bean_bowl_input_from_scan(tiles, inp, true, true);
    let dry = fill_bean_bowl_input_from_scan(tiles, inp, false, true);
    fill_bean_bowl_action_to_live(fill_bean_bowl_held_if_needed(&green, &dry), inp.held_id)
}

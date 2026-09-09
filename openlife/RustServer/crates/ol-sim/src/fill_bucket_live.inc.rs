// FILL-BUCKET: live scan helpers included from profession_scan.rs

/// Haxe `fillBucketIfNeeded` shortCraft radius (tank 20; source 30).
pub const FILL_BUCKET_SCAN_RADIUS: i32 = 30;

/// Convert scan tiles to craft objs for [`fill_bucket_if_needed_apply_ex`].
fn craft_world_objs_from_scan_tiles(tiles: &[ScanTile]) -> Vec<crate::get_or_craft::CraftWorldObj> {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| {
            crate::get_or_craft::CraftWorldObj::simple(t.parent_id, t.x, t.y).with_uses(if t.uses > 0 {
                t.uses
            } else {
                1
            })
        })
        .collect()
}

/// True when fillBucketIfNeeded would drop or shortCraft (profession gate is separate).
// Haxe: fillBucketIfNeeded after hasOrBecomeProfession
pub fn fill_bucket_mid_pending(
    held_id: i32,
    player_x: i32,
    player_y: i32,
    tiles: &[ScanTile],
    bucket_water_source_ids: &[i32],
) -> bool {
    use crate::get_or_craft::{
        effective_bucket_water_source_ids, fill_bucket_if_needed_apply_ex, CraftScanFilters,
        FillBucketApply,
    };
    let objs = craft_world_objs_from_scan_tiles(tiles);
    let ids = effective_bucket_water_source_ids(bucket_water_source_ids);
    matches!(
        fill_bucket_if_needed_apply_ex(
            &objs,
            held_id,
            player_x,
            player_y,
            FILL_BUCKET_SCAN_RADIUS,
            ids,
            &CraftScanFilters::default(),
        ),
        FillBucketApply::DropHeldFullBucket
            | FillBucketApply::ShortCraft { .. }
            | FillBucketApply::ShortCraftOnSource { .. }
    )
}

/// WATERBRINGER `fillBucketIfNeeded` (max=1 mid) → drop / shortCraft / use source.
// Haxe: AiBase.fillBucketIfNeeded ~3515–3545
pub fn fill_bucket_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    farm_rt: &mut FarmProfessionRuntime,
) -> ProfessionScanTickResult {
    use crate::get_or_craft::{
        effective_bucket_water_source_ids, fill_bucket_if_needed_apply_ex, CraftScanFilters,
        FillBucketApply, EMPTY_BUCKET,
    };
    let objs = craft_world_objs_from_scan_tiles(tiles);
    let ids = effective_bucket_water_source_ids(&inp.bucket_water_source_ids);
    let apply = fill_bucket_if_needed_apply_ex(
        &objs,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        FILL_BUCKET_SCAN_RADIUS,
        ids,
        &CraftScanFilters::default(),
    );
    match apply {
        FillBucketApply::None
        | FillBucketApply::AlreadyHaveWaterBucket
        | FillBucketApply::NoSource => ProfessionScanTickResult::none(),
        FillBucketApply::DropHeldFullBucket => {
            // Haxe: dropHeldObject(0) — drop at current tile
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::DropAt {
                    x: inp.player_x,
                    y: inp.player_y,
                },
                had_action: true,
            }
        }
        FillBucketApply::ShortCraft {
            actor_id,
            target_id,
        } => farm_action_to_live_intent(
            tiles,
            inp,
            FarmAction::ShortCraft {
                actor: actor_id,
                target: target_id,
            },
            farm_rt,
        ),
        FillBucketApply::ShortCraftOnSource {
            actor_id: _,
            source_id,
            source_x,
            source_y,
        } => {
            if inp.held_id == EMPTY_BUCKET {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::UseAt {
                        x: source_x,
                        y: source_y,
                        target_id: source_id,
                        actor_id: EMPTY_BUCKET,
                    },
                    had_action: true,
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: EMPTY_BUCKET,
                        craft_if_needed: false,
                    },
                    had_action: true,
                }
            }
        }
    }
}

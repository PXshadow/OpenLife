# AI-CRAFT-LIVE-RESID / craft_multistep_resid (2026-07-29)

## Status: DONE (path-reach scan into craft live helper)

### Closed residual

Wire live **notReachable / hostile / blockedByAI** maps into multi-step craftItemHelper + GetOrCraft closest (Haxe `addObjectsForCrafting` `isObjectNotReachable` + `GetClosestObject*` hostile skip).

| Piece | Module | Notes |
|-------|--------|-------|
| `craft_item_helper_ex` | `craft_item.rs` | full helper + `CraftScanFilters` |
| `craft_item_with_runtime_scan` | same | sticky runtime + scan |
| `craft_have_set_ex_filtered` / `closest_craft_obj_dual_center_filtered` | `craft_dual_center.inc.rs` | dual-center path filters |
| `reanchor_craft_actor_near_target_filtered` | same | pile/loose re-anchor skips blocked |
| `expand_craft_item_live_opts_scan` / `_sticky_scan` | `get_or_craft.rs` | live expand + scan |
| `expand_craft_item_player_sticky_scan` | `craft_ai_sticky.rs` | player sticky + scan |
| `apply_sticky_craft_queue_tick` | `profession_scan.rs` | `ai_path_reach.blocked_coords` + `blocked_by_ai` |
| `get_or_craft_item_ex` / `closest_obj_by_id_filtered` | `get_or_craft.rs` | pure GetOrCraft blocked skip |

### Tests

- `craft_item::tests::scan_filters_skip_blocked_target_picks_alt`
- `craft_item::tests::scan_filters_block_all_actors_fails_or_seeks`
- `craft_item::tests::scan_filters_skip_blocked_product_already_have`
- `dual_center_tests::closest_dual_filtered_skips_blocked`
- `dual_center_tests::have_set_ex_filtered_skips_blocked`
- `get_or_craft::tests::get_or_craft_skips_blocked_tile_picks_alt`
- `get_or_craft::tests::closest_obj_by_id_filtered_respects_blocked`

### Residual (next)

1. npc_ai full multi-step GetOrCraft enqueue with multi-step state
2. ignoreFullPiles live multi-use map into CraftScanFilters
3. craftItemHelper specials retarget still unfiltered closest
4. Full GetCraftAndDrop adze/bucket / dynamic WaterSourceIds
5. Interrupted countDone re-queue polish (core sticky already DONE)

### Haxe anchors

- `AiBase.addObjectsForCrafting` ~7262 `isObjectNotReachable`
- `AiHelper.GetClosestObject*` notReachable + hostile
- `AiBase.isObjectNotReachable` / `isObjectWithHostilePath` ~9245–9281

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- craft_item get_or_craft dual_center
```

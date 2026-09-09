# AI-CRAFT-MULTI-RESID — specials CraftScanFilters + countDone re-queue

## Haxe

- `AiHelper.GetClosestObjectToPositionByIdsHelper` ~368–370 skips `isObjectNotReachable` / `isObjectWithHostilePath`
- `GetClosestObjectToTarget` / `GetClosestObjectToPositionHelper` ~199 (forge bias, GetCraftAndDrop pickup)
- `craftItemHelper` water ~6943, soil ~6788, fillBucket ~3539 pass `myPlayer`
- `craftItemHelper` ~6677–6685: `countDone < count` → `addTask` on product switch
- `doTimeStuffHelper` ~667–680: continue unfinished, else shift `craftingTasks`

## Rust

| Symbol | Role |
|--------|------|
| `closest_craft_obj_by_ids_filtered` / `closest_forge_craft_filtered` / `second_closest_craft_obj_filtered` / `closest_craft_obj_min_anchor_dist_filtered` / `closest_craft_obj_from_anchor_filtered` | GetClosest + scan skip |
| `retarget_water_source_ex` / soil / flat-rock / clay-bowl `_ex` | specials retarget with `CraftScanFilters` |
| `get_craft_and_drop_items_close_to_obj_ex` + adze/goose/kindling `_ex` | pickup filtered; count-near unfiltered |
| `fill_bucket_if_needed_apply_ex` | tank/source GetClosest filtered |
| `craft_item_helper_ex` specials | pass `&scan` into retarget / GetCraftAndDrop / goose stump / second-closest |
| `CraftAiRuntime.crafting_tasks` / `prepare_for_product` / `take_next_crafting_task` / `should_continue_unfinished` | NPC sticky interrupt re-queue |
| `craft_item_with_runtime_scan` | calls `runtime.prepare_for_product` first (no-op after PlayerCraftAi prepare) |

Count gates (`berry_pie_crust_blocked`, `bowl_fill_pickup_blocked`, GetCraftAndDrop already-enough) stay unfiltered.

## Tests

```
cargo test -p ol-sim --lib -- craft_item
cargo test -p ol-sim --lib -- craft_and_drop
cargo test -p ol-sim --lib -- craft_ai_sticky
cargo test -p ol-sim --lib -- get_or_craft
```

## Residual

1. npc `doTimeStuffHelper` full `craftingTasks` drain (runtime queue not consumed in npc_ai)
2. profession fillBucket live path still uses unfiltered wrapper unless caller passes `_ex`

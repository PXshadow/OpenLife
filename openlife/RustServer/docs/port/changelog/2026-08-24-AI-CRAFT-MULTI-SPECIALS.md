# AI-CRAFT-MULTI-SPECIALS — InitWaterSourceIds + GetCraftAndDrop wire

## Haxe

- `ServerSettings.InitWaterSourceIds` ~3999–4028 — Bowl of Water 382 + Clay Bowl 235 → `WaterSourceIds`; Full Bucket 660 + Empty Bucket 659 → `BucketWaterSourceIds`
- `AiBase.GetCraftAndDropItemsCloseToObj` ~893 + craftItemHelper adze/froe/goose/kindling + water retarget ~6905
- `AiBase.fillBucketIfNeeded` ~3515 — uses `BucketWaterSourceIds`

## Rust

| Symbol | Role |
|--------|------|
| `init_water_source_ids` / `init_water_source_ids_from_content` | pure InitWaterSourceIds |
| `CraftLiveExpandOpts::{water_source_ids,bucket_water_source_ids}` | empty → DEFAULT_* wells |
| `SimState::{water_source_ids,bucket_water_source_ids}` | seeded in `seed_craft_graph_from_content` |
| `craft_and_drop.inc.rs` include | GetCraftAndDrop + fill_bucket + GotoDropAnchor/DropNearAnchor |
| `craft_item_helper_ex(..., water_source_ids)` | retarget uses Init-derived list |

## Tests

```
cargo test -p ol-sim --lib -- craft_and_drop
cargo test -p ol-sim --lib -- water_
cargo test -p ol-sim --lib -- craft_item::
```

## Residual

1. specials retarget still unfiltered by CraftScanFilters
2. interrupted countDone re-queue polish

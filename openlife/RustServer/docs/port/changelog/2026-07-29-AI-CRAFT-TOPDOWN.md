# AI-CRAFT-TOPDOWN / craft_topdown (2026-07-29)

## Status: DONE (core)

### Implemented

Pure **searchBestTransitionTopDown** / **DoTransitionSearch** filters + object scan gates:

| Piece | Module | Notes |
|-------|--------|-------|
| `do_transition_search_skip_reason` / `CraftTransMeta` | `ol-sim/src/craft_topdown.rs` | aiShouldIgnore, time>120, undo-last, target=-1, product-as-input, product pile |
| reverseUseTarget full / targetMinUseFraction | same | via `CraftObjectIndex.closest_uses` |
| ignoreIfMax / igmoreIfMin | same | aiCraftMax/Min + radius≥40 min gate |
| hardened-row hoe+soil | same | 848 present → ignore 850/857+1138 |
| `CraftScanFilters` / `closest_craft_obj_filtered` | same | blocked / full_pile / nonempty_container tiles |
| `search_best_object_for_crafting_topdown` / `_ex` | same | reverse-graph + filters |
| default search wire | `craft_item.rs` | `search_best_object_for_crafting` → topdown; helper passes last_actor/target + index |

### Tests

- `craft_item::craft_topdown::*` — ignore, undo, time, full reverse-use, min/max, hardened row, scan filters, blocked search

### Residual

1. Full Haxe BFS `wantedObjs` / `craftActor` multi-hop (still reverse-graph path)
2. Live `Transition.ai_should_ignore` / ignoreIfMax from ServerSettings content patches
3. `ObjectDef.aiCraftMax` / `aiCraftMin` fields (index supplied by caller today)
4. Dual-center searchCurrentPosition; GetCraftAndDrop adze/bucket specials

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- craft_topdown
cargo test -p ol-sim --lib -- craft_item
```

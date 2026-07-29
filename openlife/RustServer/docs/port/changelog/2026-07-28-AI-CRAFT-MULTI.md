# AI-CRAFT-MULTI / craft_item_live (2026-07-28)

## Status: PARTIAL

### Implemented

Pure multi-step **craftItem** / **craftItemHelper** / **searchBestObjectForCrafting** + craftItemHelper specials first cut:

| Piece | Module | Notes |
|-------|--------|-------|
| `craft_item` / `craft_item_helper` | `ol-sim/src/craft_item.rs` | nested as `get_or_craft::craft_item` via `#[path]` |
| `search_best_object_for_crafting` | same | reverse-graph path leaf→root + world scan; radius expand 15 / +30 / max 60 |
| `FailedCraftings` | same | Haxe 15s `AiTimeToWaitIfCraftingFailed` cooldown |
| `ItemToCraftState` / `CraftAiRuntime` | same | sticky product / transActor+Target / startLocation + fail map + last_actor_id |
| `CraftLiveExpandOpts` | same | home / is_or_can_smith / now_sec for live expand |
| Forge SMITH gate | `FORGE_IDS` 303/304/305 | `NeedSmithProfession` when `is_or_can_smith=false` |
| Water / soil retarget | `retarget_water_source` / `retarget_soil_for_clay_bowl` | Clay Bowl 235 / Empty Water Pouch 209 → closest wells; soil pile/loose within 30 |
| Bowl fill anti-loops | `bowl_fill_pickup_blocked` | Gooseberries 253 / Dry Beans 1176 + last_actor_id |
| Berry pie crust gate | `berry_pie_crust_blocked` | 253+264 blocked when 265/272 count > 1 |
| Forge flat-rock / clay-bowl bias | `retarget_flat_rock_near_forge` / `retarget_clay_bowl_away_from_forge` | min dist 3 from forge |
| TIME actor | `WaitTime` | actor id -1 (PLAYER -2 fails) |
| Fire bow kindling residual | `fire_bow_needs_kindling` | 74+67 without 72/61 near shaft → SeekIngredient kindling |
| Sheep/cow second-closest | deadly actor ids | Knife/sword/mango leaf specials |
| Live expand | `expand_craft_item_live` / `_opts` / `_sticky` | wired into `resolve_seek_or_craft_live` / `_ex` |
| Intent map | `craft_item_decision_to_live_intent` | UseAt / DropAt / SeekOrCraft / pile USE |

### Tests

- `craft_item::*` — cooldown, multi-step leaf pair, held USE, forge smith, pile, home, water/soil/bowl/pie/forge/TIME specials, sticky runtime
- `get_or_craft::resolve_craft_item_multi_step_pickup_actor`
- `get_or_craft::expand_craft_item_held_actor_uses_target`
- `get_or_craft::expand_craft_item_live_ex_home_and_smith`
- `get_or_craft::sticky_runtime_cooldown_across_expand`

### Residual (still open)

1. Full Haxe `searchBestTransitionTopDown` / `DoTransitionSearch` filters (`aiShouldIgnore`, time transitions, max/min counts, undo-last, reverseUseTarget full bowls)
2. Hostile / unreachable / ignoreFullPiles on closest scan
3. ~~`Player.itemToCraft` + `failedCraftings` fields~~ → **AI-CRAFT-STICKY DONE** (`Player.craft_ai`)
4. Full `GetCraftAndDropItemsCloseToObj` bodies (adze/froe+log, water bucket/tank shortCraft)
5. Dynamic `ServerSettings.WaterSourceIds` (currently `DEFAULT_WATER_SOURCE_IDS` wells 663/662)
6. npc_ai full GetOrCraft enqueue with multi-step state
7. Dual-center searchCurrentPosition scan polish; pile-vs-loose *1.5 / r=6 re-anchor
8. Interrupted craft re-queue when countDone < count

### Haxe anchors

- `AiBase.craftItem` ~6611–6644
- `AiBase.craftItemHelper` ~6646–7130 (specials ~6750–7037)
- `AiBase.searchBestObjectForCrafting` ~7132–7186
- `AiBase.craftItemMax` ~6604
- `ServerSettings.AiTimeToWaitIfCraftingFailed` / `AiMaxSearchRadius` / `AiMaxSearchIncrement` / `WaterSourceIds`

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- craft_item
cargo test -p ol-sim --lib -- get_or_craft
```

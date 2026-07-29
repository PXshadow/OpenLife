# AI-CRAFT-DUAL / dual_center_search (2026-07-29)

## Status: DONE (pure + helper wire)

### Implemented

| Piece | Module | Notes |
|-------|--------|-------|
| `craft_obj_in_dual_center` | `ol-sim/src/craft_dual_center.inc.rs` | home-only vs home+player via `searchCurrentPosition` |
| `craft_have_set_ex` | same | dual-center have-set for reverse-graph search |
| `closest_craft_obj_dual_center` | same | rank by player distance inside dual-center radius |
| `reanchor_craft_actor_near_target` | same | pile r=6 near target; pile vs loose `*1.5` (quad); loose r=6; keep original from_pile on fall-through |
| helper wire | `craft_item_helper` | re-anchor before pile/loose acquire; seek uses `craft_have_set_ex` |
| `CraftTopDownOpts.search_current_position` | `craft_topdown.rs` | threaded from sticky `ItemToCraftState`; dual have-set + dual resolve |
| `resolve_side` / `resolve_side_filtered` | craft_item / topdown | dual membership + player-rank (filtered path + scan gates) |
| default `search_current_position` | `ItemToCraftState` | true (Haxe `IntemToCraft`); `onlyHome` forces false |
| lib re-exports | `lib.rs` | dual helpers + `CraftActorReanchor` / `ACTOR_NEAR_TARGET_R` |

### Tests

- `get_or_craft::craft_item::dual_center_tests::*` (9) — dual membership, have_set_ex, rank, pile*1.5, r=6 loose, quad
- `craft_item::tests::pile_actor_empty_hands_use_pile` — re-anchor preserves pile source when no better loose

### Residual

1. Live `notReachable` / hostile maps into dual scan (`CraftScanFilters` on live helper)
2. Haxe TODO L7250: double-count objects when home and player radii overlap
3. Dynamic pile_id from ObjectDef (caller `pile_id_for` already used)
4. Haxe L7242: live player-center scan currently TODO-disabled in Haxe; Rust honors sticky flag (default true)

### Haxe anchors

- `AiBase.searchBestObjectForCrafting` ~7132–7186
- `AiBase.addAllObjectsForCraftig` / `addObjectsForCrafting` ~7219–7361
- `AiBase.craftItemHelper` pile*1.5 / r=6 ~7050–7083
- `AiHelper.IntemToCraft.searchCurrentPosition` default true
- `craftItem` onlyHome forces searchCurrentPosition=false ~6616

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- dual_center
cargo test -p ol-sim --lib -- craft_item::
```

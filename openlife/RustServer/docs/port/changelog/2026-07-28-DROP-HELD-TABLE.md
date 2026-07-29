# DROP-HELD-TABLE / table_prefer

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** DONE (pure + live wire + tests) — applied to `drop_held_ai.rs` / `player.rs` / `lib.rs` / `npc_ai.rs`

## Haxe

- `AiHelper.ShouldDropOnTable` / `isSmallFoodToStore` / `IsBakedPie` (~30–40)
- `GetClosestObjectToPositionHelper` container prefer factors (~195, 232–272)
  - Omelette 1285 → Table 3371 factor 0.25
  - Pies / Cooked Mutton 570 → Wooden Slot Box 3065 (0.25), Basket 292 (0.5), other (0.8)
  - Same-food in container → factor ×0.5
  - Non-table/small-food: skip free-container drop-in (L195)
- `storeInQuiver` clothingObjects scan → quiver snapshot

## Rust

- Apply: `crates/ol-sim/src/_apply_drop_held_table.py` (idempotent)
- Build hook: `build_craft_live_tick` → `build_drop_held_table`
- `drop_held_ai.rs`: `should_drop_on_table`, `is_small_food_to_store`, `container_prefer_factor`,
  `closest_preferred_container`, `best_empty_or_container_drop`, `quiver_from_clothing_snapshot`
- `Player` / `PlayerSnapshot`: `clothing` + `clothing_uses` (6 slots)
- `npc_ai.rs`: fill `DropHeldSensorExtras.quiver` from snapshot clothing

## Tests

- table/small-food helpers
- omelette prefers table over closer empty
- mutton prefers wooden slot box
- pie prefers basket already holding pie
- non-table (stone) skips free container
- quiver clothing snapshot → SelfClothing

## Residuals (other chunks)

- PreferShortCraft BusyMoving wait tick (DROP-HELD-LIVE)
- Nested held_contains_clay from basket contents

## Manual apply (if build hook not yet run)

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
python crates\ol-sim\src\_apply_drop_held_table.py
cargo test -p ol-sim --lib -- drop_held_ai -- --test-threads=1
```

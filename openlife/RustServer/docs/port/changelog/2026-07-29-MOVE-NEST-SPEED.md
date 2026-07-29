# MOVE-NEST-SPEED / held_nest_mult

**Date:** 2026-07-29  
**Haxe:** `MoveHelper.calculateSpeed` L161–189 (contained objects after backpack)  
**Rust:** `ol-sim` `move_speed.rs` + `move_nest_speed_inc.rs` + live `player_move_speed` / path-start

## Behavior

Haxe order:

1. Product of backpack `containedObjects` via `calculateObjSpeedMult` (clamp \[0.6, 0.98\])
2. If both shoes: `containedObjSpeedMult = sqrt(...)` (backpack only)
3. For each `heldObject.containedObjects`: `*= calculateObjSpeedMult`
4. For each sub-object under those: `*= calculateObjSpeedMult` (one nest level only)
5. Bad-biome double mali / floor √ / horse √ on combined product

## Implementation

- Pure: `held_nest_speed_product`, `combine_backpack_and_held_nest`
- Pure: `backpack_nest_speed_product` + `resolve_backpack_speed_product` (Haxe `getPackpack` = clothing[5]; flat backpack fallback)
- `apply_held_floor_speed_ex` / `_at_ex` / `apply_calculate_speed_full` take held nest + clothing pack
- Live wire: `p.held_helper` + `p.clothing_helpers[5]` at path-start + reported speed

## Tests

- `held_nest_speed_product_*` — empty, one contained 0.8, depth-2 ignored, missing id → 0.98
- `combine_backpack_and_held_nest_shoes_sqrt_backpack_only` — shoes √ pack only
- `resolve_backpack_prefers_clothing_nest` / empty nest ignores flat
- `apply_held_floor_speed_ex_nest_biome_floor_horse_order`
- Live: `player_move_speed_held_nest_cargo_slows`, `player_move_speed_clothing_backpack_nest`, `player_move_speed_shoes_soften_backpack_not_nest`

## Residual

- Flat `Player.backpack` (SAY STORE) and clothing[5] nest are dual representations; no automatic dual-write sync
- Non-`_ex` `apply_held_floor_speed` / `_at` default `held_nest_product=1.0` (live uses full path)
- `apply_grave_curse_live_gates` on every calculateSpeed path (separate TODO)

## Apply

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- held_nest
cargo test -p ol-sim --lib -- move_speed
```

# DROP-HELD-LIVE — holding_player / selfplay quiver / npc winter

**Date:** 2026-08-30  
**Mode:** implement  
**Status:** **DONE** (core) — npc floors → **NPC-IGNORED-FLOOR**

## Haxe

- `isPickingupFood` ~8658: `getHeldPlayer()` then `dropPlayer(x,y)`
- `dropHeldObject` `storeInQuiver` from `clothingObjects`
- `isHandlingFire` Fire 82 kindling first when `TimeHelper.Season == Winter`

## Rust

| Symbol | Role |
|--------|------|
| `PlayerSnapshot.holding_player_id` | mother `heldPlayer` for npc food pickup |
| npc `DropHeldPlayer` | `SAY DROPBABY` (live PUTDOWN/DROPBABY); log `food_drop_held_player` |
| selfplay SMART-DROP | `quiver_from_clothing_snapshot` + `held_contains_clay` |
| `EnvSnapshot::is_winter` | npc profession `is_winter` via `EnvView` (poison → false) |

## Tests

- `snapshot_held_player_id_from_start_holding`
- `drop_held_player_at_range` / `drop_held_player_after_failed_object_drop`
- `env_snapshot_is_winter_case_insensitive`

```
cargo test -p ol-sim --lib -- snapshot_held
cargo test -p ol-sim --lib -- drop_held_player
cargo check -p ol-server
```

## Residual

~~npc `ignored_floor_ids` empty~~ **NPC-IGNORED-FLOOR DONE**. `smart_drop_held_profession` still default-empty quiver (npc force_drop / selfplay fill snapshot).

# 2026-09-09 SEASON-REGROW-DECAY

**status:** **DONE** (map-slice `RespawnOrDecayPlant` parity)

Haxe `TimeHelper.RespawnOrDecayPlant` (~1253) is **not** spring-only for the current object: line 1120 runs every season. The Haxe comment about winter never running applies to hidden/original extra passes, not visible plants.

Live now in `world_time.rs` `do_world_map_time_stuff_ex2`:

- **Winter multi-use:** decrement uses including the last use, then `transform_to_dummy` (bush 30 → empty 279).
- **Winter single-use:** clear the tile (`winterDecayFactor`); optionally hide for spring if `springRegrowFactor > rand`.
- **Spring hidden plants:** empty tile + hidden layer (not fleeing rabbit) surfaces with factor 2.
- **Spring empty bush:** `undoLastUseObject` path (`last_use_object` 279↔30).
- Original biome for rabbit hide is stored only when the tile is not snow (after snow spread).

Patches for `winterDecayFactor` / `springRegrowFactor` were already in `object_data_remainder.inc.rs`. Long-term original respawn + offspring from existing stay in `long_term.rs`.

Tests: `winter_clears_single_use_and_may_hide` / `spring_surfaces_hidden_plant` / `spring_regrow_and_winter_multiuse_bush_decay`.

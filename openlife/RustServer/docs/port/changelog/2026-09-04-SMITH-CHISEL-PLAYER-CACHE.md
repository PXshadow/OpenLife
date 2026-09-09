# 2026-09-04 SMITH-CHISEL-PLAYER-CACHE

**status:** **DONE**

Haxe `PatchObjectData` fills `objectIdArrays[455]` once. npc already used load-time `SteelChiselFamilyTable`. Player/sim profession ticks re-scanned ContentDb every tick.

- `SimState.steel_chisel_family` from content at `new()` (load-time)
- `seed_craft_graph_from_content` refreshes the table
- `apply_profession_ladder_tick` / `apply_profession_scan_tick` clone extras via `chisel_family_extra_from_state`

Tests: `player_path_chisel_family_uses_sim_cache` / `seed_craft_graph_from_content_transitions`

`cargo check -p ol-server --offline` ok.

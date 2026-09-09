# 2026-09-04 SMITH-LADDER-PEER-KIND

**status:** **DONE**

Haxe `countProfession(profession)` is per `do*` body. Ladder used one primary-kind `peer_count` for every step on the player/sim path (`peer_count_by_kind: None` → fallback). npc already filled the table.

- `peer_counts_by_kind_from_state` — SimState analogue of `npc_peer_counts_by_kind` (Farm/Smith/Baker/Pottery/Shepherd/FireFood/HandlingFire)
- `apply_profession_ladder_tick` / `apply_profession_scan_tick` set `peer_count_by_kind: Some(...)`
- `ladder_profession_scan_tick` already overwrites `inp.peer_count` from `peer_count_for_kind(step.kind)`

Tests: `peer_counts_by_kind_from_state_splits_smith_and_baker` / `npc_peer_count_for_kind_multi_prof_and_wounded`

`cargo check -p ol-server --offline` ok.

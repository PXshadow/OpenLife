# 2026-09-09 OWNED-GATE-DELETE

**status:** **DONE**

Haxe `WorldMap.deleteObjectHelperIfUseless` L535-537 TODO: do not delete ObjectHelper with owners (gate).

- `is_helper_to_be_deleted` matches Haxe `isHelperToBeDeleted` plus owner lists
- `World::delete_object_helper_if_useless` drops useless helpers, keeps map id
- Map-time slice (`do_world_map_time_stuff_ex`) calls it before contained timers

Tests: `owned_gate_helper_is_not_dropped` / `helper_to_be_deleted_keeps_owned_gate`

`cargo check -p ol-server --offline` Finished.

Residual: dummy-id mismatch repair not ported; `set_object_complex` still uses `is_complex()` so an empty +owned helper is not inserted that way. Next **SCORE-AGE-58**.

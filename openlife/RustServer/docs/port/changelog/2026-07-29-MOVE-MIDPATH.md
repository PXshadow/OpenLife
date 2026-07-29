# MOVE-MIDPATH / path_recon (2026-07-29)

## Haxe
- `MoveHelper.calculateNewPos` (private, L853–877) — half-step recon along path waypoints from `startingMoveTicks` × speed.
- Dead in Haxe (no callers); residual listed under timed-movement polish.

## Rust
- Pure: `calculate_new_pos`, `calculate_segment_length_haxe`, `reconcile_mid_path_tile`, `LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR`.
- `MovePath.original_waypoints` + `start_sim_time` snapshot at accept.
- Wire: `apply_move_path_start` recon before jump gate so mid-path MOVE uses progress, not only last committed tile.
- Cheat factor port-as-is (multiplied after `movedLength` → no effect).

## Tests
- `calculate_new_pos_*`, `reconcile_mid_path_tile_from_original_waypoints`, `build_move_path_stores_original_waypoints`
- Live: `mid_path_recon_commits_half_step_before_replace`

## Residual (S-MOVE)
- Held nested `containedObjects` mult
- VOG_UPDATE on CancleMovement
- World-wrap CancleMovement when |x|/|y| exceeds map size

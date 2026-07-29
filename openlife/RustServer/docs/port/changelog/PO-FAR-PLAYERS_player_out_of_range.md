# PO-FAR-PLAYERS / player_out_of_range (2026-07-28)

## Status: **DONE** (core + torus + periodic gate)

### Haxe
- `Connection.SendToMeAllClosePlayers` L393–411 — viewer-centric roster sweep + FRAME
- `Connection.sendToMePlayerInfo` L414–448:
  - skip deleted / held
  - far **and** not topLeader → `PLAYER_OUT_OF_RANGE` (PO)
  - else PU (+ PM when moving and `sendMovingPlayer`) + NAME
  - distance via `WorldMap.transformX/Y` then `isClose`
- Callers: LOGIN (`sendMoving=true`); periodic `SendMoveEveryXTicks` (`sendMoving=false`)
- Distance product: `MaxDistanceToBeConsideredAsClose` often `2_000_000` (PO rare)

### Rust
- `ol-sim/src/leadership.rs`
  - `is_close_pu` / **`is_close_pu_wrap`** (torus via `math_wrap::wrap_delta`)
  - `decide_player_info_range` / **`decide_player_info_range_wrap`**
- `ol-sim/src/leader_range.rs`
  - pure `PlayerInfoSubject` / `ViewerSubjectAction` / `decide_viewer_subject` / **`decide_viewer_subject_wrap`**
  - `collect_far_non_leader_p_ids` / **`collect_far_non_leader_p_ids_wrap`**
  - live `send_to_me_all_close_players` (reads world wrap dims) / `pu_close_max_distance`
  - **`should_refresh_close_players`** / **`SEND_MOVE_EVERY_X_TICKS=-1`** / **`send_to_me_all_close_players_all_viewers`**
- `ol-sim/src/math_wrap.rs` — declared module; shared wrap helpers
- Wire: LOGIN `send_to_me_all_close_players(..., true)`; tick gate when `SEND_MOVE_EVERY_X_TICKS > 0`
- Protocol: `ol-protocol::format_player_out_of_range` (`PO\np_id …\n#`)

### Tests
- pure: `decide_viewer_subject_*` / `collect_far_non_leader_*` / `max_distance_zero` / **`is_close_pu_wrap_torus_*`** / **`decide_*_wrap_torus_*`** / **`should_refresh_close_players_gate`**
- live: `send_to_me_all_close_players_po_for_far_non_leaders` / **`…_torus_edge_not_po`** / **`…_send_moving_false_skips_mover_pu`** / `pu_close_max_distance_broadcast_all`

### Residuals
- LiveSettings `MaxDistanceToBeConsideredAsClose` product 2e6 (intentional practical cull = `NEARBY_RANGE`)
- `SEND_MOVE_EVERY_X_TICKS` is a const (not hot-reload LiveSettings); product default stays disabled
- NAME body: Haxe lineage full name vs Rust first+family

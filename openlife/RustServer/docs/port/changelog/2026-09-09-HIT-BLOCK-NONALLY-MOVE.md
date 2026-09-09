# 2026-09-09 HIT-BLOCK-NONALLY-MOVE

**status:** **DONE**

Haxe `GlobalPlayerInstance.killHelper` L4369 TODO: block movement if not ally (with weapon). Haxe still only slows; Rust implements the TODO intent.

- Same sensors as speed mali: `getClosePlayer(1.5, hostile, weapon)` / `has_close_hostile_with_weapon`
- New MOVE (`apply_move_path_start` + instant `apply_move_deltas_with_seq`) returns `MoveReject::CloseHostileWeapon`
- Allies with weapons still MOVE; unarmed non-allies still MOVE
- Speed mali (`angryTime < 0`) unchanged

Tests: `move_blocked_when_close_armed_nonally` / `move_allowed_when_close_armed_ally` / `move_allowed_when_close_unarmed_nonally`

`cargo check -p ol-server --offline` Finished.

Residual: an already-accepted path is not auto-cancelled until the next MOVE (reject CancleMovement). Next **BABY-BONES-ARMS**.

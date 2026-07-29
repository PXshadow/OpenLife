# MOVE-VOG-WRAP / cancel_wrap (2026-07-29)

## Haxe
- `MoveHelper.CancleMovement` (L698–724) — `useTeleport` → `VOG_UPDATE` at relative `p.x p.y`, then map chunk + forced PU + waitForForce.
- `MoveHelper.moveHelper` (L550–586) — if birth-relative `|x|≥width` or `|y|≥height`, fold once by ±map size and `CancleMovement(p, seq, true)`.
- Jump/blocked/rate-limit: `CancleMovement(p, seq, quadDist > 25)`.
- Empty path / forceStop / mid-path block: default `useTeleport=true`.

## Rust
- Pure: `fold_relative_around_world`, `fold_world_pos_around_world`, `cancel_should_use_vog`, `CANCEL_VOG_QUAD_THRESHOLD` (25).
- `MoveReject::WorldWrapped` (silent after cancel; `cancel_with_vog`).
- `cancel_movement(..., use_vog)` sends `format_vog_update` (birth-relative) + `force_send_map_chunk` + force PU.
- `apply_move_path_start` world-wrap gate before jump check.
- MOVE hard-reject: EmptyPath always VOG; JumpTooFar/BlockedStart/JumpRateLimited use `cancel_should_use_vog(jump_quad)`.
- forceStop / mid-path blocked cancel use `use_vog=true`.

## Tests
- `fold_relative_around_world_one_fold_per_axis`, `fold_world_pos_via_birth_origin`, `cancel_vog_quad_threshold`
- Live: `cancel_movement_use_vog_sends_vu`, `cancel_movement_no_vog_skips_vu`, `world_wrap_cancels_with_vog_on_move_start`, `world_wrap_no_fold_when_relative_inside_map`

## Residual (S-MOVE)
- Held nested `containedObjects` mult
- Connection MaxDistance broadcast fans
- exactTx/exactTy explicit reset + calculateSpeed restamp beyond path clear (partial via force PU speed restamp)

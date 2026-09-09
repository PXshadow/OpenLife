# 2026-09-09 HIT-STOP-MOVE

**status:** **DONE**

Haxe `GlobalPlayerInstance.killHelper` L4368 TODO: stop movement if hit.

- Connecting DoDamage (`HitResult::Wound` / `Kill`) on SAY HIT and protocol `KILL` calls `cancel_movement` on the **target** (server pos + forced PU + human `waitForForce`)
- Miss / too-far / unarmed-ally first-hit warn do not cancel
- Stationary targets are left alone (no extra CancleMovement)

Tests: `say_hit_connecting_cancels_target_path` / `say_hit_miss_does_not_cancel_target_path` / `say_hit_ally_warn_does_not_cancel_target_path`

`cargo check -p ol-server --offline` Finished.

Residual: **HIT-BLOCK-NONALLY-MOVE** (L4369 hard-block move vs armed non-ally; speed mali already live). Next **HIT-BLOCK-NONALLY-MOVE**.

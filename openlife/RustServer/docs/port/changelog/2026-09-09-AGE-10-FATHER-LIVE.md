# 2026-09-09 AGE-10-FATHER-LIVE

**status:** **DONE**

Haxe TimeHelper L780-805: when `Std.int(trueAge) == 10` and the child still follows mother, maybe `setFollowPlayer(father)`.

- `tick_vitals` calls `try_age10_father_refollow_on_cross` once when `true_age` crosses 10
- Follow father (existing `set_follow`) + LEADER/FOLLOWER pins
- Child public `I FOLLOW MY FATHER!`; father private `MY SON/DAUGHTER NAME FOLLOWS ME NOW!`
- Child HAPPY + father HUBBA PE
- Chance still Haxe `rand > 0.4` male / `0.8` female

Tests: `age10_father_refollow_follow_say_once` / `father_refollow_pure_gates`

`cargo check -p ol-server --offline` Finished.

Residual: Haxe re-rolls for the whole year 10 while still on mother; Rust fires once on the integer-year cross. Lineage myEveId/generation + SendFollowingToAll not in this leftover. Next **SPRINGS-TAR-RESPAWN**.

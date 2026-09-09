# 2026-09-09 RECENT-EXILE-ALLY

**status:** **DONE**

Haxe `GlobalPlayerInstance.kill` L4525 TODO: count as ally if exile happened not long ago (both sides).

- `SocialState.exile` stamps `(exiler, target)` sim_time (`exile_times`)
- `is_ally` still true for `RECENT_EXILE_ALLY_SECS` (30) after that stamp, either direction
- HIT/kill ally-warn and ally prestige use that `is_ally`
- Persist / direct `exiles` insert (stamp 0 or missing) is not recent
- `get_top_leader` walk unchanged (exile still breaks the chain)

Tests: `is_ally_recent_exile_still_ally_both_sides` / `is_ally_unstamped_exile_is_stale` / `say_hit_recent_exile_still_ally_warns_stale_does_not`

`cargo check -p ol-server --offline` Finished.

Residual: Haxe never specified the window (30s chosen); `exile_times` is session-only. Next **NAME-FULL-LINEAGE**.

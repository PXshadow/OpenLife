# 2026-09-02 ANGRY-TIME-MIN

**status:** **DONE**

Haxe `TimeHelper` angryTime: `moreAngry` (killMode or last attacker armed) drains toward `CombatAngryTimeMinimum` (−60), else recover toward `CombatAngryTimeBeforeAttack` × biome (river ×2, desert ×0.5). Far-clear last attacker *after* the angry step.

Live: `tick_vitals` already passed `combat_angry_time_minimum_live()` into `tick_angry_time`. This leftover proves the live floor, uses river biome recover, and matches Haxe last-attacker order.

Tests: `kill_mode_clamps_to_live_combat_angry_time_minimum` / `vitals_river_biome_recovers_angry_faster` / `vitals_recovers_and_kill_mode_drains`.

`cargo check -p ol-server --offline` ok.

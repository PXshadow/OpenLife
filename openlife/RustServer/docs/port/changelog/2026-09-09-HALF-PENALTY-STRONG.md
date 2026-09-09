# 2026-09-09 HALF-PENALTY-STRONG

**status:** **DONE**

Haxe `MoveHelper.calculateSpeed` L162 TODO: half contained-speed penalty for strong.

- `is_strong` = Noble / King / Emperor (`SpeedPrestigeClass::is_strong`, same as `Lineage.isNobleOrMore`)
- Live path-start MOVE and `player_move_speed` (PU/FX) no longer hardcode `false`
- Formula unchanged: `1 - (1-p)/2` when contained_mult < 1

Tests: `half_penalty_for_strong_halves_slowdown` / `speed_prestige_class_is_strong_noble_plus` / `player_move_speed_noble_half_contained_penalty`

`cargo check -p ol-server --offline` Finished.

Residual: none for this leftover (Haxe never implemented the TODO body; Rust maps it to Noble+). Next **HIT-STOP-MOVE**.

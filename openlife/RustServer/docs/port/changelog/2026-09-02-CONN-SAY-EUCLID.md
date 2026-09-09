# 2026-09-02 CONN-SAY-EUCLID

**status:** **DONE**

Haxe `sendSayToAllClose` uses `isClose` → `CalculateDistance` squared-Euclidean (torus wrap). SAY already fanned via `nearby_conn_ids` / `is_close_pu_wrap`. Residual was Chebyshev comments and no diagonal proof.

Live: `say_close_range` uses `GameplayKnobs.max_distance_say`. Diagonal (15,15) Chebyshev 15 would hear; Euclidean ~21.2 does not.

Tests: `say_adult_diagonal_outside_euclidean_close_for_say` / `say_adult_uses_live_max_distance_say`.

`cargo check -p ol-server --offline` ok.

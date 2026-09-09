# 2026-09-08 FILL-BERRY-HELD

**status:** **DONE**

Haxe mid `fillBerryBowlIfNeeded(true)` onlyFillHeld (~627) before bean held.

- Held Bowl of Gooseberries **253** (not full) → USE closest wild/domestic bush r=20
- Empty hands skip (Haxe BOWLFILLER path is `return false` TODO)
- Ladder `FILL_BERRY_HELD` first on MidPriorityTasks / CriticalMisc

Tests: `fill_berry_bowl_held_if_needed_uses_closest_bush` / `farm_profession_scan_tick_fill_berry_held_uses_bush` / `apply_profession_scan_tick_fill_berry_held_uses_held_bowl`

`cargo check -p ol-server --offline` ok.

Residual: `shortCraft(0, 400, 10)` ~626. Next **PULL-CARROT-ROW**.

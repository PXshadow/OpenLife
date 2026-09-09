# 2026-09-08 FILL-BEAN-HELD

**status:** **DONE**

Haxe mid `fillBeanBowlIfNeeded(*, true)` onlyFillHeld (~628–629) before `isHandlingFire`.

- Green held 1175 → USE plant 1173, then dry held 1176 → USE plant 1172
- No pickup / GetItem(235)
- Ladder `FILL_BEAN_HELD` first on MidPriorityTasks / CriticalMisc

Tests: `fill_bean_bowl_held_if_needed_green_then_dry` / `farm_profession_scan_tick_fill_bean_held_green_then_dry_skips_pickup` / `apply_profession_scan_tick_fill_bean_held_uses_held_green`

`cargo check -p ol-server --offline` ok.

Residual: `fillBerryBowlIfNeeded(true)` ~627. Next **FILL-BERRY-HELD**.

# 2026-09-08 FILL-BEAN-BOWL

**status:** **DONE**

Haxe `AiBase.fillBeanBowlIfNeeded()` low green-bean slot after `doCarrotFarming(1)`.

- Held Bowl of Green Beans 1175 (not full) → USE on Green Bean Plants 1173
- Green fill requires dry stock `[1176, 1172] >= 1`
- Pickup close bowl / `GetItem(235)` when best BowlFiller
- Ladder `FILL_BEAN_BOWL` after `DO_CARROT_LOW` on LowPriorityWork + AgeRotatedJob

Tests: `fill_bean_bowl_if_needed_green_held_and_pickup` / `farm_profession_scan_tick_fill_bean_bowl_uses_held_on_plant` / `apply_profession_scan_tick_fill_bean_bowl_uses_held_green`

`cargo check -p ol-server --offline` ok.

Residual: mid `fillBeanBowlIfNeeded(*, true)` onlyFillHeld ~628. Next **FILL-BEAN-HELD**.

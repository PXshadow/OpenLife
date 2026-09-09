# 2026-09-09 PULL-CARROT-ROW

**status:** **DONE**

Haxe mid `shortCraft(0, 400, 10)` (~609) before `fillBerryBowlIfNeeded(true)`.

- Closest carrot row **400** within Chebyshev **r=10**
- Empty hands → USE; held → dropHeld; no `hasOrBecomeProfession`
- Seed guard via existing `short_craft_apply` (`!hasCarrotSeeds && uses < 4`)
- Ladder `PULL_CARROT_ROW` first on MidPriorityTasks / CriticalMisc

Tests: `pull_carrot_row_if_needed_empty_hand_drop_and_seed_guard` / `farm_profession_scan_tick_pull_carrot_row_uses_empty_hand_r10` / `apply_profession_scan_tick_pull_carrot_row_uses_empty_hand`

`cargo check -p ol-server --offline` ok.

Residual: second `shortCraft(0, 400, 10)` after `isMakingSeeds` ~635. Next **DIE-SCORE-SKIP** (done same fire).

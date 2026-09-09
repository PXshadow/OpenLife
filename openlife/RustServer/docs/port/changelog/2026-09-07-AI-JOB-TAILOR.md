# 2026-09-07 AI-JOB-TAILOR

**status:** **DONE**

Haxe assigned/last **TAILOR** `craftHighPriorityClothing` + `craftMediumPriorityClothing(100)` + `craftLowPriorityClothing(100)` (no age gates). Clothing SM + **CLOTHING-HAS-TAILOR** reused.

- `ProfessionScanKind::Tailor` on assigned/last ladder
- `plan_assigned_tailor_clothing`; medium also if assigned/last (not only `age > 10`)
- Player/npc sticky from `assignedProfession` / `lastProfession`
- Live scan `tailor_profession_scan_tick` → CraftItem / SeekOrCraft / SELF

Tests: `assigned_tailor_runs_medium_under_age_10` / `apply_profession_scan_tick_crafts_clothing_assigned_tailor`

`cargo check -p ol-server --offline` ok.

Residual: mid `isFeedingPlayerInNeed()` max=1 (SMITH&lt;1). Next **AI-FEED-MID**.

# 2026-09-05 AI-JOB-COLLECT

**status:** **DONE**

Haxe `AiBase.isCollecting`: `hasOrBecomeProfession('COLLECTOR', max)` then wrap maxSearchRadius=60 and `isCollectingHelper` — kindling makeOrCollect, keepBushesAlive, age>40 doSmithing(1), rabbits, mutton shortCraft, pork, thread, extra kindling. Assigned/last `isCollecting(100)`; after critical `isCollecting(1)`.

- `collecting.rs` pure SM + `collecting_live.inc.rs` scan
- Assigned COLLECTOR sticky; LowPriorityWork before age-rotated
- Player `collector_profession` / npc `collector_rt`

Tests: `collecting::*` / `apply_profession_scan_tick_collects_kindling_near_home`

`cargo check -p ol-server --offline` ok.

Residual: craft-index `countCurrentObject` vs scan count; WATERBRINGER `doWatering`.

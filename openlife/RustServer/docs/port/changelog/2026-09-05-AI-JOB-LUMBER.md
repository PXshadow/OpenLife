# 2026-09-05 AI-JOB-LUMBER

**status:** **DONE**

Haxe `AiBase.isCuttingWood`: firePlace required, `hasOrBecomeProfession('LUMBERJACK', max)`, Firewood 344 near fire (count r=15, GetCraftAndDrop max 10 dist 8), then Butt Log 345 near home (max 5 dist 8). Assigned/last `isCuttingWood(100)`; low-priority `isCuttingWood()`.

- `cutting_wood.rs` pure SM + `cutting_wood_live.inc.rs` scan
- Assigned LUMBERJACK sticky; LowPriorityWork after age-rotated
- Player `lumberjack_profession` / npc `lumberjack_rt`

Tests: `cutting_wood::*` / `apply_profession_scan_tick_cuts_firewood_near_fire`

`cargo check -p ol-server --offline` ok.

Residual: `cleanUp()` after stage 3; COLLECTOR `isCollecting`.

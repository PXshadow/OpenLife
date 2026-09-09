# 2026-09-05 AI-JOB-WATER

**status:** **DONE**

Haxe `AiBase.doWatering` assigned/last **WATERBRINGER**: `hasOrBecomeProfession('WATERBRINGER', 100)` then closest dry target r=30 (`GetClosestObjectToPositionByIds`), carrot-stock skip, `doWateringOn` Bowl of Water 382. Helper miss after a target zeros `profession['WATERBRINGER']`.

- Farm WaterBringer reused (no new crate). Mid `doWatering(3)` list-order kept.
- Assigned max 100; live scan closest helper.
- Tests: `do_watering_helper_closest_*` / `do_watering_closest_assigned_max_*` / `farm_profession_scan_tick_assigned_waterbringer_*` / `apply_profession_scan_tick_waters_dry_carrots_assigned_waterbringer`

`cargo check -p ol-server --offline` ok.

Residual: `fillBucketIfNeeded`; low-priority `doWatering(1)`; mid list-order vs Haxe closest.

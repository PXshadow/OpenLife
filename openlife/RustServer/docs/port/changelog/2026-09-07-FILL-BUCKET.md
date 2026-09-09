# 2026-09-07 FILL-BUCKET

**status:** **DONE**

Haxe `fillBucketIfNeeded` (WATERBRINGER max=1) after graves, before craft queue. Pure SM reused; live mid ladder + scan.

- Held full/partial bucket → drop at feet
- Tank + empty bucket shortCraft; else empty bucket on well/source
- `mid_tasks_pending` when fill would act

Tests: `fill_bucket_tank_and_source` / `apply_profession_scan_tick_fill_bucket_drops_held_full`

`cargo check -p ol-server --offline` ok.

Residual: low `doWatering(1)`. Next **YOU-ARE-PROF**.

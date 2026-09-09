# 2026-09-07 DO-WATERING-LOW

**status:** **DONE**

Haxe `AiBase.doWatering(1)` low WATERBRINGER slot. Assigned `doWatering(100)` and mid `doWatering(3)` already live.

- `hasOrBecomeProfession('WATERBRINGER', 1)` then closest dry r=30 (carrot-stock skip)
- Ladder `DO_WATERING_LOW` after collecting on LowPriorityWork; prefix on AgeRotatedJob (before jobByAge / unassigned NPC)
- FILL_BUCKET WaterBringer peer-cap now max=1 (Haxe `fillBucketIfNeeded`)

Tests: `do_watering_closest_low_max_refuses_one_peer` / `farm_profession_scan_tick_low_watering_waters_closest_and_peer_caps` / `apply_profession_scan_tick_waters_dry_carrots_low_watering`

`cargo check -p ol-server --offline` ok.

Residual: makeFood `doWatering(1)`; mid list-order vs Haxe closest. Next **DO-CARROT-LOW**.

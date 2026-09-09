# 2026-09-08 DO-CARROT-LOW

**status:** **DONE**

Haxe `AiBase.doCarrotFarming(1)` low CARROTFARMER slot. Assigned `doCarrotFarming(100)` stays ASSIGNED_JOB.

- `hasOrBecomeProfession('CARROTFARMER', 1)` then existing `do_carrot_farming`
- Ladder `DO_CARROT_LOW` after `DO_WATERING_LOW` on LowPriorityWork; prefix on AgeRotatedJob (before jobByAge)
- Live scan maps `DO_CARROT_LOW` → CarrotFarmer (not assigned max=100)

Tests: `do_carrot_farming_low_max_refuses_one_peer` / `farm_profession_scan_tick_low_carrot_pulls_row_and_peer_caps` / `apply_profession_scan_tick_pulls_carrot_row_low_carrot`

`cargo check -p ol-server --offline` ok.

Residual: doCriticalStuff `doCarrotFarming(1)`. Next **FILL-BEAN-BOWL**.

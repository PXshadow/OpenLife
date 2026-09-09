# 2026-09-05 AI-JOB-GRAVE

**status:** **DONE**

Haxe `AiBase.isHandlingGraves` / GRAVEKEEPER. Assigned/last `isHandlingGraves(100)`; mid after fire; hungry graves then fire. `getBestAiForObjByProfession('GRAVEKEEPER', grave)` (old AIs eligible). Dig ids `[87,88,89,357]`; search r=30; shovel `GetItem(502,10)` then hoe 850.

- `handling_graves.rs` pure SM + `GraveKeeperProfessionRuntime` / lastGrave
- Live scan + ladder + npc; player writeback
- Best-AI from closest grave (player + npc)
- Residual: own-grave `getOwnerAccount` fill (sensor false live)

Tests: `handling_graves::*` / `apply_profession_scan_tick_grave_keeper_drops_bones_basket` / `is_self_best_grave_keeper_from_state_closer_weight_wins` / `apply_profession_scan_tick_far_peer_not_best_skips_new_grave`

`cargo check -p ol-server --offline` ok.

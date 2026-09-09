# 2026-09-04 FIRE-BEST-AI

**status:** **DONE**

Haxe `getBestAiForObjByProfession('FIREKEEPER', home|firePlace)` is a distance pick. Live scan used sticky/weight/`peer_count < 1`. BowlFiller popcorn already had `is_self_best_bowl_filler`.

- `FireKeeperPeer` + `is_self_best_fire_keeper_for_obj` — others without **weight > 0** skipped; self without weight +100 quad; closest remaining wins
- `ProfessionScanInput.is_best_fire_keeper_at_home/fire` filled from SimState / npc snapshots
- handling-fire scan uses those flags (not last/weight/peer heuristic)
- Lose pick zeros `profession['FIREKEEPER']` weight (Haxe `this.profession=0`)

Tests: `self_is_best_fire_keeper_when_no_peer_has_weight` / `closer_fire_keeper_peer_wins_distance_pick` / `is_self_best_fire_keeper_from_state_closer_weight_wins` / `handling_fire_scan_uses_best_ai_flags_not_peer_heuristic` / `apply_profession_scan_tick_far_peer_not_best_skips_new_fire`

`cargo check -p ol-server --offline` ok.

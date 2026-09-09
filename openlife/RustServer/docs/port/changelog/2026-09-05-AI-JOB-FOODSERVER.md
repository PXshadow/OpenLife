# 2026-09-05 AI-JOB-FOODSERVER

**status:** **DONE**

Haxe `AiBase.isFeedingPlayerInNeed` assigned/last **FOODSERVER**: age/food gates, sticky `feedingPlayerTarget`, `GetCloseStarvingPlayer` r=30, 40%/80% full, `hasOrBecomeProfession(100)`, then feed held or `SearchBestFood(target, self)`, goto/feed.

- `feeding_player.rs` pure SM + `feeding_player_live.inc.rs` scan
- Player `foodserver_profession` / npc `foodserver_rt`
- Live `try_do_eating` / SearchBestFood

Tests: `feeding_player::*` / `apply_profession_scan_tick_feeds_starving_assigned_foodserver`

`cargo check -p ol-server --offline` ok.

Residual: mid `isFeedingPlayerInNeed()` max=1 (SMITH&lt;1); YOU ARE name; npc starving cands empty.

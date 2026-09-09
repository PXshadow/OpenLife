# 2026-09-07 AI-FEED-MID

**status:** **DONE**

Haxe mid `doStuff && profession['SMITH'] < 1 && isFeedingPlayerInNeed()` (default maxPlayer=1). Assigned FOODSERVER(100) already live.

- `FEED_PLAYER_IN_NEED` ladder step → FoodServer max=1
- Sensors `feed_player_need` / `smith_blocks_feed` (stage >= 1)
- Live `apply_profession_scan_from_sensors` applies FeedPlayerInNeed

Tests: `mid_feed_max1_peer_cap_and_smith_gate` / `apply_profession_scan_tick_feeds_starving_mid_max1`

`cargo check -p ol-server --offline` ok.

Residual: YOU ARE name; npc starving cands empty. Next **FILL-BUCKET**.

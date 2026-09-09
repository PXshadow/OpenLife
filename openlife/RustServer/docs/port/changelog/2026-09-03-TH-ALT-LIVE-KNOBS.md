# 2026-09-03 TH-ALT-LIVE-KNOBS

**status:** **DONE**

Haxe `FortificationCosePerHit` (default 1) is live on `server.toml` / LiveSettings / `GameplayKnobs`. Fortify USE cost is `floor(fortValue * fortification_cost_per_hit)`. AlternativeOutcome percent/hits knobs were already live.

Test: `fortify_apply_uses_live_cost_per_hit` (cost 2 → 4 coins on shaft value 2).

`cargo check -p ol-server --offline` ok.

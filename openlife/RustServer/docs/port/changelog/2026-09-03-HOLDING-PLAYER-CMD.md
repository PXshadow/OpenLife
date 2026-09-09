# 2026-09-03 HOLDING-PLAYER-CMD

**status:** **DONE**

Haxe `TransitionHelper.doCommandHelper` `heldPlayer != null` (before neverDrop):

- USE/REMV: `dropPlayer(player.x, player.y)` at carrier feet, then refuse (no tile trans / no REMV take).
- DROP/SWAP keep click-tile `dropPlayer` (`GPI.drop` / `GPI.swap`).

Tests: `use_holding_player_drops_at_feet_and_refuses` / `use_holding_player_drops_at_feet_not_click` / `use_intent_holding_player_drops_at_feet` / `remv_holding_player_drops_at_feet_and_refuses`.

`cargo check -p ol-server --offline` ok.

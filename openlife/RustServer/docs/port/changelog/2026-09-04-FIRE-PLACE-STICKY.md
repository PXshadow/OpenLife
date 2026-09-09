# 2026-09-04 FIRE-PLACE-STICKY

**status:** **DONE**

Haxe `GlobalPlayerInstance.firePlace` is GetCloseFire + tile refresh, then null on isHandlingFire give-up. Live scan recomputed GetCloseFire every tick and never wrote GPI.

- `resolve_fire_place` — sticky tile if occupied, else GetCloseFire 83→346→82→85
- `commit_fire_place` / `write_player_fire_place` → `Player.ai_fire_place_*`
- handling-fire sensors take sticky coords; apply ladder/scan + npc write back
- give-up (ashes / non-fire tile) nulls firePlace (GO HOME uses home)

Tests: `resolve_fire_place_prefers_sticky_tile` / `is_handling_fire_fallthrough_gives_up_place` / `apply_profession_scan_tick_writes_fire_place_sticky` / `apply_profession_scan_tick_clears_fire_place_on_give_up`

`cargo check -p ol-server --offline` ok.

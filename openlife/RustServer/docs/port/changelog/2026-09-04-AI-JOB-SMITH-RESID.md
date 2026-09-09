# 2026-09-04 AI-JOB-SMITH-RESID

**status:** **DONE** (live GetCraftAndDrop USE/DROP I/O)

Haxe `GetCraftAndDropItemsCloseToObj(forge, id, want, dist)`: live smith scan always passed `craft_drop_near_count=0` / `nearby_exists=false`, so Pickup never fired and stock-at-cap still crafted.

- Scan fill: `CountCloseObjects(forge, id, dist)` + `GetClosestObjectToTarget(..., 30, dist)`
- dist 5 (flat/stone) / 10 (crucible 319)
- Pickup coords = object tile (npc DROP-on-object pickup)
- Held close: `dropHeldObject(dist, forge)` empty tile, not ON forge

Tests: `smith_craft_drop_live_counts_and_pickup` / `smith_craft_drop_live_held_goto_and_empty_drop` / `smith_crucible_craft_drop_uses_dist_ten`

`cargo check -p ol-server --offline` ok.

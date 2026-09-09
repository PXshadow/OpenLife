# 2026-09-04 DROP-HELD-QUIVER

**status:** **DONE**

Haxe `dropHeldObject` → `storeInQuiver` scans `clothingObjects`. Profession DropHeld used `smart_drop_held_profession_ex` with an empty quiver (selfplay/npc force-drop already filled snapshot).

- `ProfessionScanInput.clothing` / `clothing_uses` from player snapshot (npc + live scan builders)
- `smart_drop_held_profession_ex` → `quiver_from_clothing_snapshot` into `DropHeldSensorExtras.quiver`
- farm/smith/pottery/baker DropHeld pass clothing through

Tests: `smart_drop_held_profession_stores_yew_bow_in_clothing_quiver` / `smart_drop_held_profession_empty_clothing_does_not_store_bow` / `profession_scan_input_clothing_snapshot_feeds_quiver`

`cargo check -p ol-server --offline` ok.

# 2026-09-02 GRAVE-INFO

**status:** **DONE**

Haxe `Connection.sendGraveInfo` / `sendGraveInfoHelper`: client `GRAVE x y` → `GO` (GRAVE_OLD).

Lookup tile helper `livingOwners[0]` / `owner_id`; silent if no lineage. Body: `x y p_id po_id death_age underscored_name lineage` (`createLineageString(false)`). Empty name is `~`. Death age is floor((sim_time − deathTime) / 60).

Tests: `grave_query_sends_grave_old` / `create_lineage_string_eve_and_child` / `grave_old_name_and_dead_since`.

`cargo check -p ol-server --offline` ok.

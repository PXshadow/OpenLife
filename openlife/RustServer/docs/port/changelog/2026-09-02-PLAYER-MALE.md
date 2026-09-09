# 2026-09-02 PLAYER-MALE

**status:** **DONE**

Haxe `GPI.isMale` / `isFemale` is `ObjectData.getObjectData(po_id).male` (default false). Rust used display id 19 / name heuristic unless `race != 0 || male=true`, so `male=0` females missed the flag.

Live: `content_person_is_female` always uses `ObjectDef.male` when the person object exists. Wired: fertility, HIT male damage, soul view, twin mother pick, lock USE, death score. Eve race collect accepts `ObjectDef.male` via `collect_person_ids_for_race_ex`.

Tests: `player_is_female_uses_object_def_male` / `person_is_female_male_false_is_female_without_heuristic` / `race_person_collect_and_pick`.

`cargo check -p ol-server --offline` ok.

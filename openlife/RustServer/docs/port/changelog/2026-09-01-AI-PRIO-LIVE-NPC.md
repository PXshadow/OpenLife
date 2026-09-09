# 2026-09-01 AI-PRIO-LIVE npc sensors + BowlFiller + animal path deadly

**status:** **DONE** (npc live sensors / popcorn peer / path deadly-for-me)

## Closed

- **AI-PRIO-LIVE:** `npc_ai` now fills `LiveSensorInput` from `AnimalWorld` + `GetCloseDeadlyPlayer` snapshots and runs Haxe `escape` before eat/food/jobs. Carried babies (`held_by`) skip think.
- **YOU ARE gender:** live `plan_do_naming_you_are_ex_gender` (prior turn) — female list vs male list.
- **FEVER-EMOTE live:** `apply_update_emotes_tick` on `tick % 30` (prior turn).
- **AI-ANIMAL-GOTO residual:** `is_deadly_animal_for_path_for_player` uses Haxe `isAnimalDeadlyForMe` (loved biome + weapon). Collect/next-step `_for_player` helpers exist.
- **AI-MAKE-STUFF popcorn:** `is_self_best_bowl_filler` + live `ProfessionScanInput.is_best_bowl_filler`.
- **PlayerSnapshot:** `angry_time` / `is_cursed` / `display_object_id` / last-attack ids for sensors; clothing craft uses `ObjectData.male`.

## Tests

- `npc_ai::live_sensors_escape_wolf_near_npc`
- `npc_ai::deadly_player_candidate_marks_attacker_unfriendly`
- `pathfind::loved_biome_animal_not_deadly_for_path_unless_weapon`
- `fire_food_profession::self_is_best_bowl_filler_*` / popcorn skip when not best
- `player::snapshot_includes_angry_curse_and_display_for_ai_sensors`

## Still not in scope / polish

- PHOTO / VOG / multi-server twins / SQL parked
- ~300 unused ModuleConst knobs not Live
- Profession Defer* polish tails (farm chain, smith USE/DROP I/O, nested milk)
- NPC profession `npc_try_walk_to` still uses simplified animal footprints (escape/food can take `_for_player` later)

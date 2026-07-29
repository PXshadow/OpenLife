# C-SS-MIN-AGE-AI / min_age_ai

**Date:** 2026-07-29  
**Status:** DONE (core residual live paths)

## Summary

Wire live `ServerSettings.MinAgeToEat` (`GameplayKnobs.min_age_to_eat`, default 3) into residual paths that still used ModuleConst duplicates after **C-SS-MORE-BATCH3** (eat/feed/prestige already live).

## Haxe

- `ServerSettings.MinAgeToEat = 3`
- Callers: AI (`AiBase` child/hungry/follow/profession count), `placeGrave` baby 3053, map pins BABY, birth aging mult, UpdateEmotes hunger

## Rust

| Area | Change |
|------|--------|
| Grave | `resolve_place_grave_id_with_min_age` + `place_grave_for_conn` uses `state.gameplay.min_age_to_eat` |
| Map pins | `send_baby_map_pin_to_parent` live min age |
| Profession | `peer_count_for_kind` + `age_job_pending_ex` + job sensor flags live + `sensors_from_ext_ex` in ladder tick |
| AI ladder | `sensors_from_ext_ex` / `is_child_and_has_mother_ex` / `LiveSensorInput.min_age_to_eat` |
| AI follow | sticky clear/bands/acquire `_ex` + live.inc wire |
| Aging | `birth_cross_species_aging_mult_ex` + `player_birth_aging_mult` live |
| Fever PE | `UpdateEmotesInput.min_age_to_eat` from gameplay |

## Residual

- Clothing MinAge self-equip gate remains **commented in Haxe** (not ported as active)
- Other ~190 ServerSettings ModuleConst knobs (unrelated)

## Tests

- `select_grave_live_min_age_override` / `place_grave_live_min_age_baby_threshold`
- `is_child_and_has_mother_ex_live_boundary` / `sensors_from_ext_ex_live_min_age_food_gates`
- `plan_follow_sticky_clear_ex_live_min_age` / `follow_max_tiles_for_context_ex_live_baby_hungry`
- `age_job_pending_ex_live_min_age` / `job_sensor_flags_from_sticky_ex_live_min_age`
- `birth_cross_species_aging_mult_ex_live`

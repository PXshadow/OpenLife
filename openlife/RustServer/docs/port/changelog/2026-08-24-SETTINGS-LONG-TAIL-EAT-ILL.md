# SETTINGS-LONG-TAIL / AllowEatingOrFeedingIfIll + ResistanceAgainstFeverForEatingMushrooms live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (eat ill-gate + mushroom fever resist live; long-tail continues)
- **Haxe:** `AllowEatingOrFeedingIfIll = false`; `ResistanceAgainstFeverForEatingMushrooms = 0.2`
- **Rust:** LiveSettings → GameplayKnobs → `try_eat_held` (`feeder_may_eat_or_feed` + `apply_drugs_fever_resistance`)

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. **AllowEatingOrFeedingIfIll** on live USE eat (yellow-fever gate; live MinAgeToEat)
3. **ResistanceAgainstFeverForEatingMushrooms** on live isDrugs eat (`yellowfeverCount` + fever `timeToChange`)
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- Next used live-path ModuleConst (HIT / food leftovers)
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs try_eat_held_blocks_yellow_fever try_eat_held_drugs_uses_live
```

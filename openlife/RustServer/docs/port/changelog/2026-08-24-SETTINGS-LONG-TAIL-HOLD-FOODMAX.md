# SETTINGS-LONG-TAIL / HOLD pickup trio + food_max death/starve live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (two live leftover groups; long-tail continues)
- **Haxe:** `MaxAgeForAllowingClothAndPrickupFromOthers = 10`; `PickupExhaustionGain = 0.2`; `PickupFeedingFoodRestore = 1.5`; `DeathWithFoodStoreMax = -0.1`; `FoodStoreMaxReductionWhileStarvingToDeath = 5`

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. HOLD/doBaby pickup knobs on live `HOLD` in `lib.rs`
3. Food_max death line on live HIT / animal-path kill; starve reduction on `calculate_food_store_max_ex`
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- Drink / MaxJumps still not fully live in `lib.rs` SELF/MOVE
- Animal pop loved-biome dying factor not on live `tick_animals_dt` (wander, not pop)

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs live_pickup_age_and_restore_knobs live_starve_reduction_overrides_module
```

# SETTINGS-LONG-TAIL / MinSpeedReductionPerContainedObj + LovedFoodUseChance live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (two live leftovers promoted; long-tail continues)
- **Haxe:** `MinSpeedReductionPerContainedObj = 0.98`; `LovedFoodUseChance = 0.5`
- **Rust:** LiveSettings → GameplayKnobs → `contained_obj_speed_mult_ex` / `evaluate_loved_food_extra_ex`

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. **MinSpeedReductionPerContainedObj** on live calculateSpeed contained clamp
3. **LovedFoodUseChance** on live USE loved-plant extra harvest
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- Drink / MaxJumps still not fully live in `lib.rs` SELF/MOVE

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-move-rules --lib -- contained_obj_speed_mult_ex_live_clamp
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs live_loved_food_use_chance_zero_always_extra
```

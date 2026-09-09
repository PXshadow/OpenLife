# SETTINGS-LONG-TAIL / TemperatureImpactBelow + TemperatureImpactColorFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (super-hot/cold thresholds live; long-tail continues)
- **Haxe:** `TemperatureImpactBelow = 0.6`; `TemperatureImpactColorFactor = 0.5`
- **Rust:** LiveSettings → GameplayKnobs → `is_super_hot_for_person_ex` / `is_super_cold_for_person_ex` on `soul_view_for`

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. **TemperatureImpactBelow** + **TemperatureImpactColorFactor** on live AI soul super-hot/cold
3. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- Next used live-path ModuleConst (scan food/HIT leftovers)
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs super_hot_cold soul_view_super_hot
```

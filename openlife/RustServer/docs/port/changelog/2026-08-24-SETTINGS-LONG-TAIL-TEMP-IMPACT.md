# SETTINGS-LONG-TAIL / TemperatureImpactPerSec + IfGood + TemperatureInWaterFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (body-heat impact knobs live; long-tail continues)
- **Haxe:** `TemperatureImpactPerSec = 0.03`; `TemperatureImpactPerSecIfGood = 0.06`; `TemperatureInWaterFactor = 1.5`
- **Rust:** LiveSettings → GameplayKnobs → `body_heat_step_ex` / `update_player_temperature_ex` on tick_vitals

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. **TemperatureImpactPerSec** / **IfGood** / **TemperatureInWaterFactor** on live `updateTemperature` body heat
3. Wired MAP-TEMP-PLAYER `update_player_temperature_ex` into `tick_vitals` (writes `Player.heat` + `last_temperature`)
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- **TemperatureImpactBelow** / **TemperatureImpactColorFactor** on live `soul_live` super-hot/cold
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs body_heat_step_ex_live tick_vitals_body_heat old_age_increases_food_drain
```

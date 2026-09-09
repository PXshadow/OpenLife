# SETTINGS-LONG-TAIL / TemperatureReductionPerDrinking + MaxStoredWater + MaxJumpsPerTenSec live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (two live leftovers promoted; long-tail continues)
- **Haxe:** `TemperatureReductionPerDrinking = 0.5`; `MaxStoredWater = 1`; `MaxJumpsPerTenSec = 10`
- **Rust:** LiveSettings → GameplayKnobs → SELF drink / MOVE jump gate + jumpedTiles decay

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. **TemperatureReductionPerDrinking** + **MaxStoredWater** on live SELF `doSelf` drink
3. **MaxJumpsPerTenSec** on live `apply_move_path_start` rate-limit / jump cost and `tick_vitals` decay
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- **TemperatureImpactPerSec / TemperatureImpactPerSecIfGood / TemperatureInWaterFactor** on live `heat_ideal` / `update_player_temperature` (tick_vitals)
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs drink_ jump_rate decay_jumped apply_move_path
```

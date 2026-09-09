# SETTINGS-LONG-TAIL / CloseGraveSpeedMali + TemperatureSpeedImpact live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (vitals calculateSpeed remaining knobs promoted; long-tail continues)
- **Haxe:** `CloseGraveSpeedMali = 0.9`; `TemperatureSpeedImpact = 1`; `MoveHelper.calculateSpeed`
- **Rust:** LiveSettings → GameplayKnobs → `VitalsSpeedLiveKnobs` → `vitals_speed_product_live`

## Implemented this fire
1. Both knobs on ServerConfig / LiveSettings / GameplayKnobs
2. FIELD_MAP Live
3. `grave_curse_speed_factor_ex` + live temp impact on `vitals_speed_product_live`
4. Tests: `grave_curse_speed_factor_ex` + `apply_live_settings_gameplay_knobs`

## Residual
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-move-rules --lib -- grave_curse_speed
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs
```

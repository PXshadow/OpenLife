# SETTINGS-LONG-TAIL / SpeedWithBothShoes live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (SpeedWithBothShoes promoted; long-tail continues)
- **Haxe:** `ServerSettings.SpeedWithBothShoes = 1.1`; `MoveHelper.calculateSpeed` when `hasBothShoes && !onHorseOrCar`
- **Rust:** `LiveSettings.speed_with_both_shoes` → `GameplayKnobs` → `VitalsSpeedLiveKnobs` → `apply_calculate_speed_full_live` / `player_move_speed`

## Implemented this fire
1. `speed_with_both_shoes` on ServerConfig / LiveSettings / GameplayKnobs (default 1.1)
2. FIELD_MAP `SpeedWithBothShoes` → Live
3. `shoes_speed_factor_ex` + floor-core live path uses the knob
4. `player_move_speed` now uses live vitals knobs (same as path-start)
5. Tests: `shoes_speed_factor_ex_live_override` + `live_speed_with_both_shoes_knob`

## Residual
- Next: `AgingFactorWhileStarvingToDeath` / `GrownUpAge` (tick_vitals age_step still ModuleConst)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- SpeedWithBothShoes speed_with_both_shoes
cargo test -p ol-move-rules --lib -- shoes_speed_factor_ex_live_override
cargo test -p ol-sim --lib -- live_speed_with_both_shoes_knob apply_live_settings_gameplay_knobs
```

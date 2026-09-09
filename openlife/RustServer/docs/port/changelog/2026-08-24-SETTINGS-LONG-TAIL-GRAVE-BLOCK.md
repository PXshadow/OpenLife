# SETTINGS-LONG-TAIL / GraveBlockingDistance live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (grave-curse distance + population gate promoted; long-tail continues)
- **Haxe:** `ServerSettings.GraveBlockingDistance = 40`; `MaxPlayersBeforeActivatingGraveCurse = 0`; `MoveHelper.calculateSpeed` curse enter/clear
- **Rust:** `LiveSettings.grave_blocking_distance` + `max_players_before_activating_grave_curse` → `GameplayKnobs.grave_curse_live_knobs()` on live speed + CU gates

## Implemented this fire
1. Both knobs on ServerConfig / LiveSettings / GameplayKnobs (defaults 40 / 0)
2. FIELD_MAP GraveBlockingDistance + MaxPlayersBeforeActivatingGraveCurse → Live
3. `live_move_speed_gates` + `apply_grave_curse_live_gates` read live knobs
4. Tests: `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover: **CombatAngryTimeBeforeAttack** (player spawn `angry_time` + fever_pe soft combat; TimeHelper recovery still ModuleConst)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs
```

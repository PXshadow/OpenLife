# SETTINGS-LONG-TAIL / MaxDistanceToBeConsideredAsCoseForMovement live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (CoseForMovement promoted; long-tail continues)
- **Haxe:** `ServerSettings.MaxDistanceToBeConsideredAsCoseForMovement = 30`; `Connection.SendMoveUpdateToAllClosePlayers` L732–743
- **Rust:** `LiveSettings.max_distance_cose_for_movement` → `GameplayKnobs` → `movement_range` / PM fan in `apply_move_path_start`

## Implemented this fire
1. `max_distance_cose_for_movement` on ServerConfig / LiveSettings / GameplayKnobs (default 30)
2. FIELD_MAP `MaxDistanceToBeConsideredAsCoseForMovement` → Live
3. Path-start PM uses `movement_range` (not PU `nearby_range` 24)
4. Test: Chebyshev 25 gets PM at 30, not at live 10; 40 never gets PM

## Residual
- Haxe `SendLocationSaysToAllClosePlayers` is debug (`DebugSayPlayerPosition=false`); Rust LS remains self-only
- Next: `MaxDistanceToBeConsideredAsCloseForSayAi` (AI hear 20)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- MaxDistanceToBeConsideredAsCoseForMovement max_distance_cose_for_movement
cargo test -p ol-sim --lib -- pm_fan_uses_live_cose_for_movement apply_live_settings_gameplay_knobs
```

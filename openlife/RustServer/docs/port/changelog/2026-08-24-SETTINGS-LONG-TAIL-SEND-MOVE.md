# SETTINGS-LONG-TAIL / SendMoveEveryXTicks live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (SendMoveEveryXTicks promoted; long-tail continues)
- **Haxe:** `ServerSettings.SendMoveEveryXTicks = -1`; `TimeHelper.DoTimeStuff` L132–135 `sendToMeAllClosePlayers(false, false)`; LOGIN `Connection.SendToMeAllClosePlayers(player, true)`
- **Rust:** `LiveSettings.send_move_every_x_ticks` → `GameplayKnobs` → `maybe_refresh_close_players` / LOGIN `send_to_me_all_close_players(..., true)`

## Implemented this fire
1. `send_move_every_x_ticks` on ServerConfig / LiveSettings / GameplayKnobs (default `-1`, Haxe product off)
2. FIELD_MAP `SendMoveEveryXTicks` ModuleConst → Live
3. Sim loop TimeHelper gate uses the live knob (`sendMoving=false`)
4. LOGIN roster sweep restored (`sendMoving=true`)
5. Tests: apply_live key + `maybe_refresh_close_players_uses_live_send_move_every_x_ticks`

## Residual
- `MaxDistanceToBeConsideredAsCoseForMovement` (Haxe typo, 30) still hardcoded on PM / LOCATION_SAYS fans
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- NAME body lineage quality (PO-FAR residual)

## Verify
```powershell
cargo test -p ol-config --lib -- SendMoveEveryXTicks send_move_every_x_ticks live_settings
cargo test -p ol-sim --lib -- maybe_refresh_close_players_uses_live_send_move_every_x_ticks apply_live_settings_gameplay_knobs
```

# MOVE-GRAVE-ALL-PATHS / grave_speed_all

**Date:** 2026-07-29  
**Haxe:** `MoveHelper.calculateSpeed` L210–231 (`isCursed` enter/clear + speed mali); callers:
- `calculateNewMovements` finalSpeed (path start) L806/836/847
- `updateMovement` path done L375
- `CancleMovement` L713  
**Rust:** `ol-sim` `move_live_gates.rs` + `lib.rs` `apply_grave_curse_live_gates` / `live_move_speed_gates` / `player_move_speed`

## Behavior

Haxe `calculateSpeed` always re-evaluates grave proximity:
1. Near bones (`hasCloseBlockingGrave` default distance) and population gate open → speed × `CloseGraveSpeedMali`, `isCursed=true`, CU/PE/say on enter
2. Outside default but still inside 1.5× clear band while cursed → keep cursed, **no** speed mali
3. Beyond 1.5× band → clear `isCursed`, CU level 0 + happy emote/say

## Implementation

- Pure already DONE (**S-MOVE-LIVE-GATES**): `resolve_grave_curse`, `has_close_blocking_grave`, fitness
- Speed mali already on all report paths via `player_move_speed` → `live_move_speed_gates`
- Mutation + CU/PE/say (`apply_grave_curse_live_gates`) now on **all** Haxe calculateSpeed wire sites:
  - path start (`apply_move_path_start`) — prior
  - path finish (`tick_move_paths` finished branch) — this chunk
  - cancel (`cancel_movement` / CancleMovement) — this chunk

## Tests

- `cancel_movement_applies_grave_curse_enter`
- `cancel_movement_clears_grave_curse_when_far`
- `path_finish_applies_grave_curse_enter` (walk into fitness band)
- Existing: `player_move_speed_live_gates_*` / `apply_grave_curse_live_gates_clear_hysteresis` / hysteresis band

## Residual

- Flat↔nest backpack dual-write (MOVE-NEST-SPEED)
- Connection MaxDistance fans (S-MOVE residual)
- Global cursedGraves index (CURSED-GRAVES-INDEX; local bone-grave scan used)

## Apply

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
python docs/port/_apply_move_grave_all_paths.py
cargo test -p ol-sim --lib -- grave_curse
cargo test -p ol-sim --lib -- cancel_movement_applies
cargo test -p ol-sim --lib -- path_finish_applies_grave
```

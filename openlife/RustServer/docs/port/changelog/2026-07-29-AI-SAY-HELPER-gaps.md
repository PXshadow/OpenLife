# AI-SAY-HELPER gap-close / scripted_cmds

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core residuals closed; JUMP BW open)

## Closed this chunk

- **MOVE!/FOLLOW pathfind** — already wired via `try_ai_follow_path_to` + `ally_goto_speaker_xy` (**AI-FOLLOW-WALK**); documented.
- **GO HOME far** — pure `should_path_to_home` / `go_home_goal_xy` / `go_home_debug_say` / `go_home_move_target`; live pathfind + `GOING HOME!` / `I CANNOT GO HOME!` when `ai_debug_say`.
- **HOME! SearchNewHome swamp** — `home_oven_biome_allowed` skips swamp without floor; local radius 80.
- **HOME! firePlace** — `Player.ai_fire_place_{id,x,y}` via `get_close_fire` after SearchNewHome.
- **STOP waitingTime assign** — `waiting_time_set` can lower prior wait (Haxe `waitingTime = 10`).
- **DROP deferred** — scripted DROP sets `ordered_to_drop` only; `tick_ordered_ai_drop` applies feet drop next vitals tick.

## Residual

- Full JUMP baby BW packet path
- GO HOME `this.time +=` mapped to `waiting_time_min` (no separate AiBase think clock)
- Global oven registry vs local scan (r=80 partial vs Haxe `WorldMap.ovens`)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_say_helper -- --test-threads=1
```

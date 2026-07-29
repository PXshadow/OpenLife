# AI-FOLLOW-WALK / continuous_follow

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core continuous walk + ally Goto pathfind)

## Scope

Close residual from **AI-LLM-APPLY** / **AI-SAY-HELPER**: continuous `isMovingToPlayer` walk toward sticky `Player.ai_follow_p_id`, and ally/LLM/scripted `Goto(speaker)` pathfind (not force-stop-only).

## Haxe

- `AiBase.isMovingToPlayer` (~8284–8324)
- `AiBase.startFollowingPlayer` Goto(speaker+1)
- `doTimeStuffHelper` auto-clear ordered follow 5 min + age clear (~560–568, ~595)
- Ally post-say / MOVE! `Goto(player.tx+1)`

## Rust

- `ol-sim/src/ai_follow_walk.rs` — pure sticky clear, distance gate, stand-off goal, ally xy
- Nested under `ai_llm_apply::ai_follow_walk` + crate re-export of decide/plan/ally helpers
- `ai_follow_walk_live.inc.rs` included from `lib.rs` → `tick_ai_follow_walk` + `try_ai_follow_path_to`
- Vitals: `tick_ai_follow_walk` after `tick_llm_speech_wire`
- LLM apply: `follow_player` / `goto_speaker` → pathfind to speaker+1
- Scripted FOLLOW/MOVE!: pathfind to speaker+1 (not force-stop-only)
- `PlayerSnapshot.ai_follow_p_id` / `ai_auto_stop_follow` for NPC
- `npc_ai` follow_walk before profession scan

## Residuals

- `AutoFollowPlayer` closest-human acquire when sticky empty
- Child-mother `getFollowPlayer` auto-assign
- Debug say target name while walking
- Specialized Haxe distance bands (baby hungry 5/3, child nice 2/4, wounded 2)
- Full Haxe random stand-off each repath (deterministic seed used)
- Mid-move Haxe `time+=1` + still `gotoAdv` vs Rust skip-repath when moving (documented delta)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_follow_walk -- --test-threads=1
cargo test -p ol-sim --lib -- try_ai_follow_path_to -- --test-threads=1
```

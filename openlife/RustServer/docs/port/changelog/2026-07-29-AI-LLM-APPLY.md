# AI-LLM-APPLY / llm_actions

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core live apply)

## Scope

Live-wire `ApplyAiResponsePlan` after LLM HTTP result in `tick_llm_speech_wire`:

- emote → PE (`doEmote` seconds unused in Haxe)
- `followPlayer` → sticky `Player.ai_follow_p_id` = **speaker** (Haxe incorrectly passed self)
- `drop` → `ai_ordered_to_drop` + stop + waiting 1s + immediate `apply_drop` at feet
- `makeItem` → resolve id/name/alias → `craft_ai.do_make_craft_command(..., silent=true)`

## Haxe

- `AiHandler.parseAiResponse` L468–506
- `AiBase.startFollowingPlayer` / `doDropCommand` / `doMakeCraftCommand`
- `GlobalPlayerInstance.findObjectByCommand` + `ObjectData.GetObjectByName`

## Rust

- `ol-sim/src/ai_llm_apply.rs` — pure resolve + sticky apply
- `Player.ai_follow_p_id` / `ai_auto_stop_follow` / `ai_follow_started_sim_time` / `ai_ordered_to_drop`
- `tick_llm_speech_wire` live apply after `plan_speech_llm_complete`

## Intentional deltas

| Topic | Haxe | Rust | Why |
|-------|------|------|-----|
| follow target | `startFollowingPlayer(aiPlayer)` self | speaker p_id | schema "walk with the player"; Haxe bug |
| bare makeItem | `findObjectByCommand` needs ≥2 tokens | bare id/name ok | LLM JSON `makeItem:"knife"` |

## Residual

- Ally Goto(speaker) pathfind (force-stop clear only)
- Continuous follow walk tick (npc uses sticky)
- Scripted sayHelper HOLA/FOLLOW/… → **AI-SAY-HELPER**
- Full live RelationshipView fields

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_llm_apply -- --test-threads=1
cargo test -p ol-sim --lib -- plan_apply_parsed -- --test-threads=1
```

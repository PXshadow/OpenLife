# AI-SAY-HELPER / scripted_cmds

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core scripted cmds)

## Scope

Haxe `AiBase.sayHelper` scripted speech commands before LLM fallback:

- HOLA / HELLO / HI — weapon/angry gates, cooldown 4s, `HOLA {name}`
- NAME? — self name + family
- ARE YOU AI / AI? — random reply table
- NICE? / JUMP! / MOVE! / NHOME!
- FOLLOW ME / FOLLOW / COME — follower/close-relative gate → sticky follow
- STOP FOLLOW / STOP / WAIT / DROP — ally gates + ordered drop
- GO HOME / HOME! — near check + SearchNewHome oven pick
- MAKE / CRAFT — ally + `do_make_craft_command` (non-silent)
- DEBUG / PROF ON-OFF / PROFESSION? / `{PROF}!` assign

## Rust

- `ol-sim/src/ai_say_helper.rs` — pure `plan_scripted_say_helper`
- `fan_out_ai_say_scripted` in `lib.rs` — live before `fan_out_ai_speech_llm`
- `Player.ai_debug_say` / `ai_debug_profession` / `ai_is_nice_baby`
- Build wire: `build_ai_say_helper.rs` via `build_do_commands` piggyback

## Residual (updated 2026-07-29 gap-close)

- Full JUMP baby BW parity
- GO HOME `this.time +=` → waiting floor (no separate AiBase.time)
- Global oven registry vs local r=80 scan
- See also `2026-07-29-AI-SAY-HELPER-gaps.md` (pathfind / DROP deferred / firePlace closed)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_say_helper -- --test-threads=1
```

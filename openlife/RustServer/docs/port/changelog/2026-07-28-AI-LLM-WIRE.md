# AI-LLM-WIRE / speech_llm

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** PARTIAL — speech hear/gate/chunk core DONE; HTTP drain → **AI-LLM-HTTP-DRAIN DONE**

## Haxe

- `Connection.sendSayToAllClose` → each AI `AiBase.say` / `sayHelper`
- Distance `MaxDistanceToBeConsideredAsCloseForSayAi` (20), attention ALL/!!/??/name/closest
- LLM fallback: human + `IsLLMActivated` + age>3 + no `!`/`?` + 4s cooldown
- Immediate `doEmote(oreally)` + `say("...")` + ally stop (`Goto` self + wait 6s)
- `AiHandler.respondToPlayerAsync` → parse → `sendResponseInChunks` → `say` + ally Goto speaker
- `checkIfYouAreAllied` silent vs loud reject

## Rust

- `ol-sim/src/ai_handler.rs` pure:
  - `collect_ai_speech_hearers` / `ai_speech_attention` / `ai_within_say_range`
  - `check_if_you_are_allied_speech`
  - `LlmSpeechRuntime` + enqueue/poll chunks + cooldown
  - `plan_speech_llm_start` / `plan_speech_llm_complete`
  - `LlmSpeechJob` / `LlmSpeechResult`
- `Player.llm_speech` sticky runtime
- `SimState.llm_speech_jobs` / `llm_speech_results`
- Live: free-form SAY → `fan_out_ai_speech_llm` (when `AI_API_KEY` set)
- Live: `tick_llm_speech_wire` each vitals — apply results, PE, chat memory, chunk SAY
- API: `take_llm_speech_jobs` / `push_llm_speech_result` for HTTP worker

## Residuals

- ~~ol-server drain~~ → **AI-LLM-HTTP-DRAIN DONE**
- AiBase scripted speech cmds (HOLA / FOLLOW / MAKE / …)
- Live apply `follow_player` / `drop` / `make_item` from parse
- Ally Goto(speaker) pathfind (force-stop cleared only)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_handler -- --test-threads=1
```

# AI-HANDLER / llm_prompt

**Date:** 2026-07-30
**Mode:** implement
**Status:** DONE (pure + env scaffold + tests; HTTP/speech residual)

## Haxe

- `AiHandler.hx` — rate limit (AiCallsPerHour / 1h window), ChatResponse retry on network errors,
  buildPrompt (soul + relationship + memory + command JSON schema), parseAiResponse (emote/actions),
  conversation log daily files, respondToPlayerAsync + sendResponseInChunks split.

## Rust

- `ol-sim/src/ai_handler.rs` — pure helpers + unit tests
- `ol-server/src/ai_llm_env.rs` — `LlmEnvConfig::from_env` (AI_API_KEY / XAI_API_KEY only)
- Build wire: `build_ai_handler.rs` via `build_do_commands` piggyback

## Residuals

- **AI-PROVIDER** — HTTP MiniMax/Anthropic-compatible call + parseResponse
- **AI-LLM-WIRE** — AiBase speech → async plan → say(chunks)
- Multi-server twin peers **parked** (out of scope)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_handler -- --test-threads=1
```

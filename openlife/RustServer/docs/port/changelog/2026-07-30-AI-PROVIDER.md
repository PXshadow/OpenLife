# AI-PROVIDER / llm_http

**Date:** 2026-07-30
**Mode:** implement
**Status:** DONE (pure + live HTTP; speech residual AI-LLM-WIRE)

## Haxe

- `AIProvider.hx` — `IsLLMActivated`, `callAi` (POST MiniMax anthropic `/v1/messages`), `parseResponse`

## Rust

- Pure (`ol-sim/ai_handler.rs`, re-export `ai_provider.rs`): `build_ai_request_body`,
  `parse_provider_response`, `ai_request_headers`, `ai_messages_endpoint`, defaults
- HTTP: `ol-server/src/ai_provider.rs` — `call_ai` / `call_ai_async` / `CallAiParams` /
  `call_ai_from_env` / `make_call_ai_inject`
- Env: `ol-server/src/ai_llm_env.rs` — `LlmEnvConfig` + boot log
- Build: `build_ai_provider.rs` via `build_do_commands` piggyback

## Secrets

- `AI_API_KEY` or `XAI_API_KEY`, `AI_API_URL`, `AI_DEFAULT_MODEL`, `AI_MAX_TOKENS_FOR_CHAT`
- Never `server.toml` (SecretOmit)

## Residuals

- **AI-LLM-WIRE** — speech → async plan → `chat_response_with` + `call_ai` inject → say(chunks)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- parse_provider -- --test-threads=1
cargo test -p ol-server -- call_ai_missing_key -- --test-threads=1
```

# AI-LLM-HTTP-DRAIN / llm_server_drain

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** DONE (source wire via build-time patcher; apply on next `cargo test`)

## Scope (narrow)

Wire ol-server main-tick drain of speech jobs through existing HTTP + pure queues:

`export_llm_speech_jobs_to_share` → `take_pending_llm_jobs_from_share` → `call_ai_async` → `push_completed_llm_result_to_share` → `import_llm_speech_results_from_share` → `tick_llm_speech_wire`

Do **not** re-audit full Haxe surface or re-implement pure parse/body. Skip multi-server twins. Secrets env only.

## Haxe

- `AiHandler.respondToPlayerAsync` — main-thread prompt; `Thread.create` + `ChatResponse`/`callAi`
- `AIProvider.callAi` — MiniMax `/v1/messages` (already **AI-PROVIDER**)

## Rust

- Build patcher: `crates/ol-sim/build_ai_llm_http_drain.rs` (piggyback `build_do_commands.rs`)
- `ol-sim/ai_handler.rs`: `LlmSpeechIoBridge` / `LlmSpeechIoShare` + take/push share helpers + `llm_speech_job_to_result`
- `ol-sim` lib: `SimState.llm_speech_io`; `import_*` / `export_*` around `tick_llm_speech_wire`; boot from `SimBootLive.llm_speech_share`
- `ol-server/ai_provider.rs`: `run_llm_speech_http_drain` + `try_drain_params_from_env` + async network retry
- `ol-server/main.rs`: create share, attach boot_live, spawn worker when `AI_API_KEY` set

## Secrets

Env only (`AI_API_KEY` / `XAI_API_KEY`, `AI_API_URL`, `AI_DEFAULT_MODEL`, `AI_MAX_TOKENS_FOR_CHAT`). Never `server.toml`.

## Residuals

- Live `ApplyAiResponsePlan` → **AI-LLM-APPLY**
- Scripted sayHelper; ally Goto pathfind; chunk `logToFile` lines; Haxe TODO `toSoul.addChatEntry`

## Follow-up (same chunk family)

- Live `logToFile` on drain worker: `log_conversation_to_file` / `_now` + `format_log_timestamp_from_unix`
- `try_drain_params_from_env` → `(params, limit, log_base)`; rate-limit and HTTP paths both log

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
# build.rs applies source patches then tests
cargo test -p ol-sim --lib -- llm_speech_job_to_result -- --test-threads=1
cargo test -p ol-server -- try_drain -- --test-threads=1
```

## Apply without full suite

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
python docs\port\_apply_ai_llm_http_drain.py
# or: docs\port\_run_ai_llm_http_drain.cmd
```

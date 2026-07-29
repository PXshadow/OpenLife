# Security notes — Open Life Rust server

## What is *not* in this tree

- **No API keys** for xAI/Grok, OpenAI, GitHub, AWS, etc.
- **No committed `.env`** or `auth.json` (game server does not use LLM providers by default).
- **No OHOL ticket server secrets** — the server only *calls* the public URL  
  `https://onehouronelife.com/ticketServer/server.php` when `verify_ohol_ticket = true`.  
  Client passwords/account keys are checked by that remote service; they are not stored as plaintext secrets in repo config.

## What *is* local / sensitive at runtime

| Item | Where | Risk |
|------|--------|------|
| SuperGrok / Grok CLI auth | `%USERPROFILE%\.grok\auth.json` (user profile, **outside** this repo) | High if leaked — never copy into the project |
| OHOL login hashes on the wire | Client → server LOGIN fields | Protocol traffic; use ticket verify + TLS reverse proxy if public |
| World / account saves | `SaveFiles/` (gitignored) | Local game state; keep off public remotes |
| `server.toml` | In tree | Public settings only (ports, paths, flags) — no private keys |

## Safe practices

1. Do **not** paste `auth.json`, Bearer tokens, or passwords into chats, issues, or commits.
2. Prefer `verify_ohol_ticket = true` for any internet-facing game port.
3. Bind `0.0.0.0` only if intentional; firewall game `8005` / web `8080` as needed.
4. If you add LLM AI later, use env vars (`XAI_API_KEY`, etc.) and `.env` (gitignored), never hardcode keys in Rust sources.
5. Before publishing this folder:  
   `rg -i "api[_-]?key|sk-|xai-|password\s*=" --glob '!content/**'`  
   and review `git status` for `.env` / `SaveFiles` / `auth.json`.
6. Haxe `ServerSettings.AiApiKey` / `Secret` are **SecretOmit** in `ol-config` field map — they must never appear in `server.toml` dumps.

## What to commit vs ignore (scripts)

| Keep in git | Leave local / gitignored |
|-------------|--------------------------|
| `scripts/start-server.ps1`, `stop-server.ps1`, `watch-server.ps1` | `scripts/patch_*.py`, `fix_*.py`, `apply_*.py` (one-shot port edits) |
| `crates/**/src/**/*.rs`, `build.rs`, `Cargo.toml`, `server.toml` | `crates/**/_patch_*`, `_apply_*`, `build_ai_*.rs` agent injectors, `*.rs.txt` snippets |
| `docs/port/*.md` (port kit when you stage it) | `SaveFiles/`, `target/`, `content/OneLifeData7/`, `.env`, `.grok/` |

One-shot agent scripts are **not** required for further development or `cargo test` once their edits are already in the Rust sources. They are ignored so they are not published by accident; delete locally anytime.

## Removed from this copy (vs OpenLifeReborn)

Development-only SuperGrok **usage-budget** scripts were **not** kept under `openlife/RustServer` — they read the local Grok CLI auth file and are unrelated to running the game server.

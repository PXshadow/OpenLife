# ohol-headless / ohol-client

Rust **One Hour One Life / Open Life** client: headless playtest binary + optional soft-FB window.

Wire protocol matches Jason Rohrer’s client and `protocol.txt`.  
**Full port kit (how it works, status, formats):** → **[docs/port/README.md](docs/port/README.md)**  
**Status / residuals:** → **[docs/port/TODO_PORT.md](docs/port/TODO_PORT.md)** (P1–P5 complete)

## Status (one glance)

| Layer | State |
|-------|--------|
| Headless wire + world + path + interact | **Playable** |
| Binary content cache (OLC1…OLS1) + optional OLSA/OLGA | **Done** |
| Soft-FB graphics (`--features gpu`) | **Playable** |
| Audio / music (`--features audio`) | **Done** (lazy banks) |
| Product pages (account / loading / death / settings) | **P5 done** (#36–39) — [TODO](docs/port/TODO_PORT.md) |

## Build / run

```powershell
cd C:\OhOl\OpenLife\RustClient
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

cargo build --release
cargo run --release --bin ohol-headless -- --self-check

# Graphical
cargo run --release --features gpu --bin ohol-client
# + device sound
cargo run --release --features "gpu,audio" --bin ohol-client
```

Without a server, `ohol-client` can open an offline demo. Headless defaults to `127.0.0.1:8005`.

## Credentials

```powershell
copy .env.example .env   # gitignored — set OHOL_EMAIL / OHOL_ACCOUNT_KEY / OHOL_PASSWORD
```

Env: `OHOL_HOST`, `OHOL_PORT`, `OHOL_CONTENT_DIR`, `OHOL_AUDIO_DISABLE`, … — [docs/port/PATHS.md](docs/port/PATHS.md).

## Content bake

```powershell
cargo run --release --bin ohol-headless -- --bake-content --src C:\OhOl\OpenLife\OneLifeData7
cargo run --release --bin ohol-headless -- --bench-load --report logs/load-bench.md
# optional large pixel atlases:
#   --bake-sprite-atlas   → cache/olsa_sprite_atlas.bin
#   --bake-ground-atlas   → cache/olga_ground_atlas.bin
```

Details: [docs/port/CONTENT_BINARY.md](docs/port/CONTENT_BINARY.md) · [docs/port/HEADLESS.md](docs/port/HEADLESS.md).

## Probes + play snapshots

```powershell
cargo run --bin ohol-headless -- --probe-move --ka
cargo run --bin ohol-headless -- --probe-actions
cargo run --bin ohol-headless -- --probe-play

# Synthetic snapshot (no server)
cargo run --bin ohol-headless -- --snapshot-self-check
# Live snapshot after login (needs server + .env)
# cargo run --bin ohol-headless -- --snapshot logs/snapshots/login.txt --snapshot-label login
```

In **ohol-client**: Settings → enable **Debug tools** (saved to `ohol_client_settings.ini`), then **F9** or bottom-right **SNAP** while playing.

Library encoders (`encode_move`, `encode_use`, …) are unit-tested and used by the binaries.

## Legal

Unofficial tooling. One Hour One Life is by Jason Rohrer. This crate reimplements the documented network protocol and client-side presentation for development and playtesting.

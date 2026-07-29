# Open Life Reborn (Rust)

Clean-room **Rust** game server for Open Life Reborn — a hardcore, seasonal
rework of [One Hour One Life](https://onehouronelife.com/)-style multiplayer,
with server-side AI NPCs and an embedded web UI (live stats + family trees).

This repository is **new and separate** from the Haxe implementation.

| | |
|--|--|
| **Game protocol port (target)** | `8005` |
| **Web UI port (target)** | `8080` |
| **Language** | Rust 2021 |
| **Database** | None (files / mmap / WAL only) |
| **Legacy reference** | `C:\OhOl\OpenLife` (not vendored here) |

## Status

Working multiplayer-oriented sim (USE/DROP/MOVE, vitals, social, self-play, web viewer). **Full Haxe parity is not claimed.**

**Systematic port kit** (architecture, dependency graphs, file matrix, TODOs, call index):

→ **[docs/port/README.md](docs/port/README.md)**

Chunk workflow: `.grok/workflows/haxe-port-chunk.rhai` (also install to `%USERPROFILE%\.grok\workflows\`).

## Quick start

```bash
# Requires Rust: https://rustup.rs
cargo test
cargo run -p ol-server
```

## Project layout

```
OpenLifeReborn/
  AGENTS.md                 # Grok rules + budget (mandatory)
  docs/
    GROK_BUILD.md           # Session playbook, /usage budget
    USAGE_BUDGET.md         # Spend log
    architecture/           # Full rebuild plan
    research/               # Extracted facts from legacy code
  crates/
    ol-protocol/            # Wire protocol (pure)
    ol-metrics/             # Counters / health
    ol-server/              # Binary
  content/                  # How to point at game data (not committed)
  web/static/               # Future site assets
```

## Architecture (short)

1. **Simulation** is the only writer of world + live players.
2. **Network / AI** submit **intents**; they do not lock the world to think.
3. **Large worlds** use chunks (hot / warm / cold), not one giant flat array.
4. **Web** reads snapshots and append-only lineage indexes.
5. **Metrics** everywhere so the server can be evolved with evidence.

Full plan: [`docs/architecture/RUST_SERVER_REBUILD_PLAN.md`](docs/architecture/RUST_SERVER_REBUILD_PLAN.md)

## Grok Build budget

Subscription usage is limited. **Always check with:**

```
/usage
```

Then fetch + gate (same API Grok Build `/usage` uses):

```powershell
.\scripts\usage-budget\fetch-usage.ps1
.\scripts\usage-budget\check-budget.ps1
```

**Manual + loop plan:** [`scripts/usage-budget/MANUAL.md`](scripts/usage-budget/MANUAL.md) · [`scripts/usage-budget/GROK_BUILD_LOOP.md`](scripts/usage-budget/GROK_BUILD_LOOP.md)

Stay under **80%** of allowance, paced until refresh.
Also: [`docs/GROK_BUILD.md`](docs/GROK_BUILD.md) · [`docs/BUILD_BACKLOG.md`](docs/BUILD_BACKLOG.md)

## License

MIT — see [LICENSE](LICENSE).

Game content (objects, sprites, etc.) remains under its original One Life / pack
licenses when you link external data; this repo does not vendor that tree by default.

## Credits

- Jason Rohrer — One Hour One Life
- Open Life / Open Life Reborn Haxe server authors and community
- This Rust tree — clean rebuild for performance, threading, and scale

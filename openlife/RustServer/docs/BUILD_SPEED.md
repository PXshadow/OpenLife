# Faster server builds

## What we configured

| Change | Effect |
|--------|--------|
| `profile.dev` `debug = "line-tables-only"` | Faster link; enough for backtraces |
| `profile.dev` `codegen-units = 256` | More parallel codegen on huge crates |
| `profile.dev.package."*"` `opt-level = 2` | Deps built once, faster runtime/tests |
| `profile.dev.build-override` `opt-level = 2` | Faster ol-sim `build.rs` patch pass |
| `.cargo/config.toml` `linker = "rust-lld"` | Faster link vs MSVC `link.exe` |

## Commands

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer

# Full server binary (dev)
cargo build -p ol-server

# Typecheck only (no link of bin) — fastest feedback
cargo check -p ol-server
# alias:
cargo ck

# Sim only
cargo check -p ol-sim
cargo test -p ol-sim --lib -- <filter>
```

## Process (often bigger than compiler flags)

1. **Max 2 concurrent workflows** that rebuild `ol-sim` (see `docs/port/QUEUE.md`). Parallel Acts thrash one `target/`.
2. Prefer **`cargo check`** for “does it compile?”; full **`cargo test -p ol-sim --lib`** only when needed.
3. Use a **filter** on tests (`cargo test -p ol-sim --lib -- jump_bw`) so you don’t run 2000+ tests every Act.

## Structural cost (harder fixes)

- `crates/ol-sim/src/lib.rs` is apply/tick (~0.5 MB). Writer tests: **`src/lib_tests.rs`** (`cargo test -p ol-sim --lib`).
- No crate `build.rs` wirers (archived in `_wire_archive/`).
- **AI pure policy** already lives in smaller crates (edit without full sim when not shadowed):
  - `ol-ai-pathing` — path-reach maps
  - `ol-ai-helper` — goals + priority ladder
  - `ol-ai-crafting` — craft graph/value
  - `ol-ai-professions` — profession pure SMs
  - `ol-ai-api` / `ol-player-helper` / `ol-main-ai`
- **Win condition:** `ol-sim` re-exports pure modules (`pub use ol_ai_pathing::…`) so rustc does not typecheck duplicate multi-kLOC copies on every sim change. Path-reach + craft_graph/value are first re-export targets (`docs/design/OL_AI_SPLIT.md`).
- **Clear sim modules (before more crates):** do **not** mix concerns — edit the right file:
  - Temperature → `temperature_handler.rs` (not food / world_time)
  - Food eating → `food_eating.rs` (not temperature / world_time)
  - Player vs world tick → see `player_tick.rs` vs `world_time.rs`
- **`ol-move-rules`:** wrap / distance / isClose / jump / **speed+nest** / **isCloseUseExact** — edit that crate without full sim when only re-exported.
  ```powershell
  cargo test -p ol-move-rules --lib
  cargo check -p ol-sim   # after changing re-export surface
  ```
- Further rule crates: `ol-transition-rules`, `ol-combat-rules` (wound plans), `ol-food-eating` (food-store-max), `ol-temperature`, `ol-social-rules`.
## First build after these settings

Expect a **one-time longer** compile while dependency crates rebuild at `opt-level = 2`. Later incremental builds should improve.

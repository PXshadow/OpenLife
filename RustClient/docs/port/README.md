# Rust client — port kit (start here)

**Repo:** `C:\OhOl\OpenLife\RustClient` (crate `ohol-headless`)  
**Living status:** [TODO_PORT.md](TODO_PORT.md) only — do not copy priority tables elsewhere.  
**Date stamp for this hub:** 2026-07-28

This folder is the **AI + human overview** of how the Open Life / OHOL **Rust client** works and how it relates to Jason’s C++ client, the Haxe client, and the Rust server. Deep format and call details live in linked docs — not duplicated here.

---

## 1. What this client is

| Mode | Binary | Feature | Role |
|------|--------|---------|------|
| **Headless** | `ohol-headless` | default | Wire, world, content, probes, bake, bench — **no window** |
| **Soft-FB GUI** | `ohol-client` | `--features gpu` | minifb window; same logic + CPU atlas + HUD |
| **Device audio** | either | `--features audio` | cpal mixer (soft-fail if no device) |

**Playable bar + P1–P5 priority list are met** (2026-07-28). Remaining work is **in-row residuals** and **deferred non-goals** only — see [TODO_PORT.md](TODO_PORT.md). Not goals: full editor suite, photo, wgpu pixel-perfect GL, C++ `folderCache` bytes.

**Sources of truth for behavior:**

| Source | Use for |
|--------|---------|
| C++ `LivingLifePage` + banks | Gameplay / wire parity |
| Haxe Open Life client | Fast load, BinPack atlas, bake patterns |
| `protocol.txt` | Wire framing and message shapes |
| This tree’s Rust modules | What we actually ship |

Paths: [PATHS.md](PATHS.md).

---

## 2. How it works (runtime)

```
                    ┌─────────────────────────────────────┐
  OneLifeData7 ──►  │ cache/*  (OLC1 OLT1 OLA1 OLS1 …)    │  prefer binary;
  (text SoT)        │ optional: OLSA / OLGA pixel atlases │  else text + rebake
                    └─────────────────────────────────────┘
                                      │
  .env / CLI ──► Login (HMAC) ──► TCP # frames ──► parse ──► Session
                                      │                      │
                                      │         ┌────────────┼────────────┐
                                      │         ▼            ▼            ▼
                                      │    LiveWorld     ClientMap    MoveState
                                      │    (players,     (MC/MX,      (path, FORCE,
                                      │     speech,       floors)      multi-MOVE)
                                      │     PE/emotes)
                                      │         │
                                      ▼         ▼
                              click_tile / pathfind ──► encode USE/DROP/MOVE…
                                      │
              ┌───────────────────────┼───────────────────────┐
              ▼                       ▼                       ▼
         soft-FB render          SoundBank              MusicBank
         (atlas, HUD,            OLSN + AIFF            OGG beds
          hover hitMap)          (+ cpal if audio)      (lewton lazy)
```

**Boot content policy:** index-only binaries first (OLS1/OLG1/OLSN/…); **pixels and PCM lazy**. Optional full pixel dumps: `--bake-sprite-atlas` (OLSA), `--bake-ground-atlas` (OLGA) — large; not default `bake_content`.

**Headed path today:** `SceneRenderer` + soft framebuffer → minifb. Not wgpu.

**Shared with server:** OLC1/OLT1 wire format via path crate `ol-binary` (`openlife/RustServer/crates/ol-binary`). Anims/sprites/sounds/ground indexes remain client-local blobs.

---

## 3. Doc map (no doubles)

| Need | Read | Not for |
|------|------|---------|
| **What’s open / done next** | [TODO_PORT.md](TODO_PORT.md) | — |
| **Playable snapshot** | [PORT_COMPLETE.md](PORT_COMPLETE.md) | Day-to-day task list |
| **Modules + topology** | [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md) | Priority rows |
| **C++/Haxe → Rust file map** | [FILE_MATRIX.md](FILE_MATRIX.md) | Full status essays (status column may lag TODO) |
| **Symbol lookup** | [CALL_INDEX.md](CALL_INDEX.md) | Architecture |
| **Binary formats + bake** | [CONTENT_BINARY.md](CONTENT_BINARY.md) | UI pages |
| **Load timings** | [LOAD_PERF.md](LOAD_PERF.md) | — |
| **Headless CLI / probes** | [HEADLESS.md](HEADLESS.md) | Soft-FB drawing |
| **Paths / env** | [PATHS.md](PATHS.md) | — |
| **How to take a chunk** | [CHUNK_PROTOCOL.md](CHUNK_PROTOCOL.md) | Current status |
| **C++ / Haxe architecture** | [ARCHITECTURE_CPP.md](ARCHITECTURE_CPP.md), [ARCHITECTURE_HAXE_CLIENT.md](ARCHITECTURE_HAXE_CLIENT.md) | Rust module inventory |
| **Dependency graphs** | [DEPENDENCY_GRAPHS.md](DEPENDENCY_GRAPHS.md) | — |
| **Map offset policy** | [MAP_GLOBAL_OFFSET.md](MAP_GLOBAL_OFFSET.md) | — |

**Rule for agents:** update **TODO_PORT** when finishing work; update **FILE_MATRIX** status cells only when touching that C++ surface; do **not** paste full P1–P5 tables into README/PORT_COMPLETE/ARCHITECTURE.

---

## 4. Module map (quick)

Single crate; main surface in `src/`. Detail + status intent: [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md).

| Area | Modules |
|------|---------|
| Wire / session | `frame`, `login`, `tags`, `parse`, `session`, `wire_log` |
| World | `live_object`, `client_map`, `move_state`, `multi_move_ext`, `map_global_offset` |
| Input / actions | `actions`, `pathfind`, `click_tile`, `rmb_action`, `hover_pick` |
| Content | `content`, `content_binary`, `category_bank`, `sprite_bank`, `anim_bank`, `ground_sprites`, `overlay_bank`, `sound_bank`, `music_bank` |
| Present | `render`, `hud`, `anim_draw`, `emotion`, `binpack`, `tga` |
| Product UI | `client_screen`, `load_progress`, `account_page` (P5 in progress) |
| Tools | `main` (headless CLI), `bin/ohol_client` (gpu), `load_bench` |

---

## 5. Commands

```powershell
cd C:\OhOl\OpenLife\RustClient
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

cargo test --lib
cargo run --bin ohol-headless -- --self-check
cargo run --bin ohol-headless -- --bake-content --src $env:OHOL_CONTENT_DIR
cargo run --bin ohol-headless -- --bench-load --report logs/load-bench.md
cargo run --features gpu --bin ohol-client
# optional: --features audio
```

Probes, bake atlases, env: [HEADLESS.md](HEADLESS.md) · [PATHS.md](PATHS.md).

---

## 6. Workflows (automation)

Prefer **≤4 concurrent** finish runs. Names live under `RustClient/.grok/workflows/` and `~/.grok/workflows/`.

| Workflow | Role |
|----------|------|
| `rust-client-priority-drive` | Walk TODO missing list |
| `rust-client-finish-slots` | One open item per run (slot filler) |
| `rust-client-p5-loading-progress` | P5#36 |
| `rust-client-p5-account-page` | P5#37 |
| `rust-client-p5-death-rebirth` | P5#38 |
| `rust-client-p5-settings-page` | P5#39 |
| `rust-client-p4-sprite-atlas-pages` | P4#40 OLSA (done; re-run only if regressed) |
| **`rust-client-play-snapshot`** | **Play-point snapshot** (CLI + F9/SNAP when `settings.debug`) |
| `rust-client-playtest-*` / `continue-*` | Playtest / load-bench loops |

Chunk process: [CHUNK_PROTOCOL.md](CHUNK_PROTOCOL.md). Active automation line: top of [TODO_PORT.md](TODO_PORT.md).

---

## 7. Content binary (one paragraph)

Text under `OneLifeData7` is authoring SoT. `bake_content` writes `cache/olc1_objects.bin`, `olt1_transitions.bin`, `ola1_anims.bin`, `olg1_ground_index.bin`, `olo1_overlays.bin`, `olsn_sounds.bin`, `ols1_sprites.bin`, `manifest.json`. Formats and flags: [CONTENT_BINARY.md](CONTENT_BINARY.md). Optional large pixel caches: OLSA sprites, OLGA ground.

---

## 8. Quality bar

- Clean modules; no LivingLifePage megafile  
- `// C++: …` anchors on non-obvious ports  
- Headless must keep working  
- Min deps (see root `Cargo.toml`)  
- No silent detail loss when porting a chunk  

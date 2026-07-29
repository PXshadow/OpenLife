# Haxe → Rust port kit (AI entry point)

**Purpose:** Systematically port every detail of the Haxe Open Life server into the Rust server (`openlife/RustServer`), file by file and chunk by chunk, without losing behavior. Code may be optimized and restructured; **semantics must not be dropped.**

**Roots**

| Role | Path |
|------|------|
| Haxe server | `C:\OhOl\OpenLife\openlife\server\` |
| Haxe AI | `C:\OhOl\OpenLife\openlife\auto\` |
| Haxe settings | `C:\OhOl\OpenLife\openlife\settings\` |
| Haxe data helpers | `C:\OhOl\OpenLife\openlife\data\` |
| Product rules | `C:\OhOl\OpenLife\TODO.MD` |
| Rust workspace | `C:\OhOl\OpenLife\openlife\RustServer\` |
| Rust sim (core) | `crates/ol-sim/` |
| This kit | `docs/port/` |

---

## Read this first (AI)

1. **[ARCHITECTURE_HAXE.md](ARCHITECTURE_HAXE.md)** — old runtime topology, ownership, locks, call graphs  
2. **[ARCHITECTURE_RUST.md](ARCHITECTURE_RUST.md)** — new crate layout, tick/net paths, module graph  
3. **[DEPENDENCY_GRAPHS.md](DEPENDENCY_GRAPHS.md)** — Mermaid graphs (Haxe + Rust + cross-map)  
4. **[FILE_MATRIX.md](FILE_MATRIX.md)** — every Haxe server file → Rust module(s) + status  
5. **[TODO_PORT.md](TODO_PORT.md)** — living done / partial / missing checklist  
6. **[HAXE_OPEN_TODOS.md](HAXE_OPEN_TODOS.md)** — open `TODO`/`FIXME` comments in legacy code  
7. **[CHUNK_PROTOCOL.md](CHUNK_PROTOCOL.md)** — how to take one chunk end-to-end (mandatory process)  
8. **[CALL_INDEX.md](CALL_INDEX.md)** — function-level index for AI lookup  

Also: [PROGRESS.md](../PROGRESS.md), [BUILD_BACKLOG.md](../BUILD_BACKLOG.md), [architecture/RUST_SERVER_REBUILD_PLAN.md](../architecture/RUST_SERVER_REBUILD_PLAN.md).

---

## Workflow (automated)

Grok Build workflow: **`haxe-port-chunk`**

| Location | Path |
|----------|------|
| Project copy (source of truth in repo) | `openlife/RustServer/.grok/workflows/haxe-port-chunk.rhai` |
| Runnable install (user Grok home) | `%USERPROFILE%\.grok\workflows\haxe-port-chunk.rhai` |

Re-copy after editing the project file:

```powershell
Copy-Item -Force `
  C:\OhOl\OpenLife\openlife\RustServer\.grok\workflows\haxe-port-chunk.rhai `
  $env:USERPROFILE\.grok\workflows\haxe-port-chunk.rhai
```

**Typical run** (workflow tool / `/workflow haxe-port-chunk`):

```text
args: {
  "haxe_file": "openlife/server/MoveHelper.hx",
  "chunk": "speed_and_path",   // optional label
  "mode": "implement"         // audit | implement | verify
}
```

Or by matrix id:

```text
args: { "matrix_id": "TH-MULTI", "mode": "implement" }
```

Each run: audit Haxe chunk → gap vs Rust → implement missing details → tests → update `TODO_PORT.md` / `FILE_MATRIX.md`.

---

## Status legend (used everywhere)

| Tag | Meaning |
|-----|---------|
| **DONE** | Behavior present + tests/evidence; parity accepted for that chunk |
| **PARTIAL** | Some paths exist; named gaps remain |
| **STUB** | Wire/API exists; body incomplete or product-postponed |
| **PURE** | Pure helpers/tests only; not fully wired into sim/net |
| **MISSING** | Not implemented in Rust |
| **NA** | Not porting (client-only, obsolete, or deliberate non-goal) |

---

## Non-negotiable rules

1. **No silent drops** — if Haxe does X, Rust must do X or document an intentional delta in `TODO_PORT.md` with reason.  
2. **Optimize structure, keep semantics** — clean modules, no giant God-files, but same outcomes on the wire and in sim.  
3. **Chunk size** — prefer 50–200 lines of Haxe logic per chunk (or one clear function family).  
4. **Tests first or with** — unit tests for pure rules; integration for protocol.  
5. **Update docs in the same change** — matrix row + TODO + call index.  
6. **Secrets stay out of git** — see `Tools/usage-budget/.env` (never commit).

---

## Suggested port order (dependencies first)

```
P0  Protocol tags + Connection bootstrap
P1  WorldMap layout + content/transitions load
P2  TimeHelper tick pipeline (player slice → world slice → long-term)
P3  TransitionHelper USE/DROP/REMV/SWAP + multi-use
P4  MoveHelper (speed, path, PM, KA)
P5  GlobalPlayerInstance: food/age/death/clothing/say
P6  Combat / baby / prestige / lineage hooks
P7  Temperature + seasons + animals (TimeHelper world)
P8  Lineage / Account / Soul / Score persistence
P9  Naming / leadership / exile details
P10 ServerAi + AiBase professions (largest)
P11 WebServer product pages
P12 ServerSettings knobs (as config, not one blob)
```

---

## Quick metrics (baseline 2026-07-26)

| Area | Haxe LOC (approx) | Rust surface |
|------|-------------------|--------------|
| `server/*.hx` | ~16k | `ol-sim` + `ol-net` + `ol-world` + `ol-server` |
| `auto/*.hx` | ~10k (AiBase alone ~7.4k) | `ai_goals`, `npc_*`, selfplay — thin |
| `ServerSettings.hx` | ~3.4k | `ol-config` + `server.toml` — partial |
| Open Haxe TODOs | ~470 matches across server/auto/settings | tracked in HAXE_OPEN_TODOS.md |

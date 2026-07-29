# Open Life Reborn → Rust Game Server Rebuild Plan

**Status:** Planning / research blueprint  
**Scope:** Game server + embedded web interface (not client)  
**Source project:** `C:\OhOl\OpenLife` (Haxe / HashLink Open Life Reborn)  
**Target:** Efficient, multi-threaded Rust server designed for large dynamic worlds, in-process AI NPCs, metrics-driven evolution, and **no traditional database**

---

## Table of contents

1. [Goals and non-goals](#1-goals-and-non-goals)
2. [Phase 0 — How to read the current architecture](#2-phase-0--how-to-read-the-current-architecture)
3. [Research findings — current architecture](#3-research-findings--current-architecture)
4. [Thread access analysis (legacy vs target)](#4-thread-access-analysis-legacy-vs-target)
5. [Rust target architecture](#5-rust-target-architecture)
6. [Large dynamic worlds without a DB](#6-large-dynamic-worlds-without-a-db)
7. [AI NPCs / players on the server](#7-ai-npcs--players-on-the-server)
8. [Web interface and lineage](#8-web-interface-and-lineage)
9. [Metrics, observability, AI-evolvable design](#9-metrics-observability-ai-evolvable-design)
10. [Implementation phases (PR-sized slices)](#10-implementation-phases-pr-sized-slices)
11. [Grok Build usage budget plan](#11-grok-build-usage-budget-plan)
12. [Risks and open decisions](#12-risks-and-open-decisions)
13. [Appendix — key source map](#13-appendix--key-source-map)

---

## 1. Goals and non-goals

### Goals

| Goal | Meaning |
|------|---------|
| **Rust rebuild of game server** | Protocol-compatible (or near-compatible) OHOL-style server written in Rust |
| **Efficiency + multi-threading** | Designed for multi-core from day one; minimal lock contention |
| **Bigger worlds** | Orders of magnitude beyond current ~500×500 flat map; still fully dynamic and simulated |
| **Server-side AI NPCs** | Rule AI + optional LLM AI run *in process*, not only as clients |
| **Web interface** | Serves site + live/historical views (players, food, scores, **family tree / character page** like lineage.onehouronelife.com) |
| **No database** | Persistence via memory-mapped / binary files, append-only journals, snapshot + WAL patterns |
| **AI-trackable evolution** | Structured metrics, events, and module boundaries so Grok/other AI can measure, propose, and verify changes safely |
| **Usage-aware development** | Work with Grok Build under a Super Grok quota that stays under ~80% of allowance, paced until refresh |

### Non-goals (for now)

- Full native client rewrite  
- Pixel-perfect 1:1 port of every Haxe quirk on day one  
- SQL / Redis / external DB dependency  
- Multi-region distributed cluster (design should not *block* sharding later, but single-node first)

---

## 2. Phase 0 — How to read the current architecture

Do **not** port line-by-line. Extract **invariants**, **data ownership**, and **protocol contracts**. Work in this order.

### 2.1 Reading order (recommended)

```
Step 1  Protocol & bootstrap
        protocol.txt
        openlife/server/Server.hx          (main, global mutex, process tags)
        openlife/server/ThreadServer.hx    (accept loop, per-connection thread)
        openlife/server/ServerTag.hx
        openlife/server/Connection.hx      (LOGIN, send helpers, KA, message queue)

Step 2  World representation & persistence
        openlife/server/WorldMap.hx        (flat vectors, wrap, save bins)
        openlife/data/object/ObjectData.hx
        openlife/data/object/ObjectHelper.hx
        openlife/data/transition/*         (TransitionImporter, TransitionData)
        OneLifeData7/objects + transitions (content, not code)

Step 3  Simulation tick
        openlife/server/TimeHelper.hx      (20 Hz, seasons, world slices, save)
        openlife/server/TransitionHelper.hx
        openlife/server/MoveHelper.hx
        openlife/server/TemperatureHandler.hx
        openlife/server/Biome.hx

Step 4  Players, lineage, accounts
        openlife/server/GlobalPlayerInstance.hx  (~5k lines — core game logic)
        openlife/server/Lineage.hx
        openlife/server/PlayerAccount.hx
        openlife/server/PlayerSoul.hx
        openlife/server/ScoreEntry.hx
        openlife/server/NamingHelper.hx

Step 5  AI
        openlife/auto/AiBase.hx            (~7k lines — professions, craft, combat)
        openlife/auto/AiHelper.hx
        openlife/auto/Pathfinder*.hx
        openlife/server/ServerAi.hx
        openlife/server/AiHandler.hx       (LLM rate limit, logs)
        openlife/server/AIProvider.hx
        openlife/auto/PlayerInterface.hx + WorldInterface.hx

Step 6  Web + product rules
        openlife/server/WebServer.hx
        WebServer/*.html
        TODO.MD                            (feature bible / Open Life Reborn rules)
        openlife/settings/ServerSettings.hx
```

### 2.2 Research checklist (fill while reading)

For each subsystem, capture:

1. **Who owns the data?** (world tiles, players, lineages, accounts)  
2. **Who mutates it?** (tick loop, connection thread, AI thread, web thread)  
3. **What locks exist today?** (global, world, player)  
4. **What is pure vs impure?** (transitions from files = pure tables; season RNG = impure)  
5. **Protocol messages in/out** for that subsystem  
6. **Save format** (bin layout, rotation `1..10`, backups)  
7. **Hot paths** (map index, transition lookup, pathfinding, PU/MX broadcast)

### 2.3 Concrete code-archaeology tasks

| ID | Task | Output artifact |
|----|------|-----------------|
| R1 | Catalog all client→server tags (`ServerTag`) and server→client tags (`ClientTag`) | `docs/research/protocol-matrix.md` |
| R2 | Document WorldMap memory layout (biomes, floors, objects, helpers, hidden layer) | `docs/research/world-layout.md` |
| R3 | Document save rotation + lineage/player/account binary formats | `docs/research/persistence.md` |
| R4 | Extract transition rules as data model (actor, target, last-use, max-use, time) | `docs/research/transitions.md` |
| R5 | Map TimeHelper phases (player, map slice, animals, seasons, save) | `docs/research/tick-pipeline.md` |
| R6 | Map AI decision graph (priority: flee, eat, feed, craft, job, follow) | `docs/research/ai-behavior.md` |
| R7 | List Open Life Reborn rule deltas vs vanilla (from TODO.MD) | `docs/research/game-rules.md` |
| R8 | Profile mentally/estimate lock hold times on USE/MOVE/AI craft | `docs/research/contention.md` |

### 2.4 Tools for research (no rewrite yet)

- Grep by tag names (`USE`, `DROP`, `KILL`, `PLAYER_UPDATE`)  
- Trace one full life: LOGIN → birth → USE berry → death → lineage write  
- Trace one AI tick: `AiBase.RunAi` → `doTimeStuff` → path → USE  
- Measure: map size (`mysteraV1Test.png` = **500×500**), AI count defaults (**40**, min **20**), tick = **1/20 s**

---

## 3. Research findings — current architecture

### 3.1 Runtime topology (Haxe today)

```
main thread
  └─ TimeHelper.DoTimeLoop()          ~20 Hz simulation
       ├─ player vitals, move resolve, temperature
       ├─ world map slice (1/WorldTimeParts of map)
       ├─ respawn / long-term decay
       └─ periodic save / backup

ThreadServer (network accept)
  └─ Thread per TCP connection
       └─ read until '#', Server.process(connection, msg)
            (often under UseOneGlobalMutex or world/player mutex)

AiBase.StartAiThread()
  └─ single AI loop over all ServerAi
       └─ doTimeStuff per AI (mutex options experimental)

WebServer thread
  └─ accept HTTP → spawn thread per request
       └─ HTML stats (players, lineage kill reasons, food, accounts)
```

### 3.2 Data model (simplified)

| Domain | Storage today | Notes |
|--------|---------------|-------|
| **World** | Flat `Vector` of length `W*H` | biomes, floors, object id arrays, ObjectHelpers, hidden objects, tile temps |
| **Map gen** | PNG color → biome id | wrap-around toroidal world |
| **Objects** | `ObjectData` from `OneLifeData7/objects/*.txt` | ~4k+ objects |
| **Transitions** | `transitions/*.txt` → multi-index maps | actor×target, last-use, max-use, reverse craft graph |
| **Players** | `GlobalPlayerInstance` in array + map | humans + AIs share same type |
| **Lineage** | `Map<id, Lineage>` in memory + bin | mother/father/eve, prestige, death reason |
| **Accounts** | email-keyed, prestige M/F, coins | binary save |
| **AI state** | per-AI targets, profession maps, craft stack | huge procedural `AiBase` |

### 3.3 Known scalability bottlenecks (why Rust redesign is needed)

1. **Full-map vectors** — O(W×H) RAM; world time walks large linear ranges; 4× map already “HUGE” in TODO.  
2. **Coarse locking** — `UseOneGlobalMutex` / `UseOneSingleMutex` / player+world mutex; connection threads can block the sim.  
3. **One AI thread** — sequential AI for all NPCs; skips ticks when slow; AI count auto-throttled.  
4. **Per-connection threads** — does not scale to thousands of sockets; better: async reactor + worker pool.  
5. **Monolithic player class** — GlobalPlayerInstance is god-object (~5k LOC).  
6. **Web rebuilds stats under player mutex** on each page load.  
7. **No structured metrics pipeline** — traces and text logs only; hard for automated evolution.

### 3.4 What to preserve (product identity)

From `TODO.MD` — not optional “flavor,” these *are* Open Life Reborn:

- Seasons / winter challenge, temperature kills, water cooling  
- Prestige-as-YUM health, classes Serf/Commoner/Noble  
- Coins, hire AI, grave-based curses, fathers  
- Generational YUM/cravings, food auto-balance  
- Server AI professions + voice commands (`MAKE`, `JOB!`, `COME`, …)  
- Reconnect + AI takes over until human returns  
- Binary saves, ~30s restart reconnect story  
- Vanilla-compatible client protocol on port **8005** (game) + web **8080**

---

## 4. Thread access analysis (legacy vs target)

### 4.1 Who needs which data (access matrix)

| Data | Sim tick | Net I/O | AI planners | Web | Save | Access pattern |
|------|:--------:|:-------:|:-----------:|:---:|:----:|----------------|
| Tile biome/floor | R/W | R (chunk send) | R | R (rare) | R | Spatial; shard by region |
| Tile objects / helpers | R/W | R/W via cmds | R/W intent | R | R | Hot; command-applied on sim |
| Transitions / ObjectData | R | — | R | R | — | Immutable after load |
| Live player body | R/W | R/W cmds | R/W intent | R snapshot | R | Entity-owned |
| Lineage / account | W on birth/death | R login | R | R heavy | R/W | Mostly append + rare update |
| AI blackboards | — | — | R/W private | R metrics | optional | Per-AI local |
| Metrics rings | W | W latency | W think time | R | flush | Lock-free SPSC/MPSC |
| Outbound protocol msgs | produce | consume/send | produce | — | — | Queues per connection |

**Rule of thumb:**  
- **Immutable content** (object defs, transitions) → shared `Arc`, no locks.  
- **World** → only sim thread(s) *commit* mutations; others submit **intents**.  
- **Players** → owned by sim; net/AI never write fields directly.  
- **Web** → always read **snapshots** or append-only lineage index, never live locks.

### 4.2 Legacy problem pattern

```
connection thread ──► lock world/player ──► mutate ──► unlock
AI thread         ──► lock               ──► path+use ──► unlock
time loop         ──► lock               ──► tick     ──► unlock
web thread        ──► lock players       ──► HTML     ──► unlock
```

Any slow AI path or HTTP stats page can stall gameplay.

### 4.3 Target concurrency model (minimal blocking)

```
                    ┌─────────────────────────────┐
                    │     Simulation Workers      │
                    │  (1+ region shards OR ECS)  │
                    │  sole writers of WorldState │
                    └────────────▲────────────────┘
                                 │ Command / Intent queue
           ┌─────────────────────┼─────────────────────┐
           │                     │                     │
    ┌──────┴──────┐       ┌──────┴──────┐       ┌──────┴──────┐
    │ Net reactor │       │ AI workers  │       │ Web (axum)  │
    │ (tokio)     │       │ (rayon/pool)│       │ read-only   │
    └──────┬──────┘       └──────┬──────┘       └──────┬──────┘
           │                     │                     │
           │  parse → Intent     │ plan → Intent       │ Snapshot bus
           ▼                     ▼                     ▼
        Outbound TX queues   Observation views    Lineage files / mmap
```

#### Design principles

1. **Single writer principle** for world tiles and player components.  
2. **Commands are data** (`Use { player, x, y, id }`, `Move`, `Say`, `AiUse`, …) validated and applied on sim.  
3. **AI never holds world write locks while thinking** — copy a **local observation** (radius RAD tiles + nearby players), plan offline, enqueue intents.  
4. **Net thread never applies transitions** — only parses and enqueues.  
5. **Snapshots for web** — every N ticks, publish immutable `Arc<WorldStatsSnapshot>` + lineage index.  
6. **Sharded world** — region grid with border ghost cells; most sim work needs only local shard lock (or lock-free ECS columns).  
7. **Prefer message passing over mutex** for cross-thread communication.

### 4.4 Suggested ownership in Rust

| Component | Owner thread / task | Sync primitive |
|-----------|---------------------|----------------|
| `ContentDb` (objects, transitions) | init once | `Arc<ContentDb>` immutable |
| `WorldShards[N]` | sim worker i | private mut; cross-shard via messages |
| `EntityStore` (players) | sim | dense ids; components SoA |
| `ConnTx[conn_id]` | net runtime | `mpsc` / `tokio::sync::mpsc` |
| `IntentQueue` | multi-producer → sim | lock-free or sharded queues |
| `AiWorkerPool` | background | work-stealing; outputs intents |
| `Metrics` | all | `metrics` crate / atomics / hdr histograms |
| `LineageLog` | sim on death/birth | append-only file + in-memory ring |
| `HttpState` | web | `watch::Receiver<Snapshot>` |

### 4.5 Blocking budget (targets)

| Path | Max hold / latency goal |
|------|-------------------------|
| Apply one player command | < 50 µs typical (no disk) |
| AI observation copy (R=32) | < 200 µs |
| AI plan (rule AI) | 0.2–2 ms off sim thread |
| World region tick slice | budget so 20 Hz holds with headroom |
| Snapshot publish | < 1 ms copy of *stats*, not full map |
| Save | background; WAL flush async; never on command path |
| HTTP character page | file/mmap read only; **0** sim locks |

---

## 5. Rust target architecture

### 5.1 Crate layout (workspace)

```
openlife-rs/
  Cargo.toml                 # workspace
  crates/
    ol-protocol/             # framing, tags, parse/serialize (# terminated)
    ol-content/              # load objects/, transitions/, categories/
    ol-world/                # shards, biomes, objects, temperature field
    ol-sim/                  # tick, transitions apply, combat, seasons
    ol-entity/               # player components, inventory, wounds
    ol-ai/                   # NPC decisions: NetIntent commands + WorldView/FoodSearch (split from ol-sim)
    ol-lineage/              # birth/death graph, prestige, character pages data
    ol-ai/                   # rule AI + pathfinding + profession FSM
    ol-ai-llm/               # optional LLM provider, rate limits (feature-gated)
    ol-net/                  # tokio TCP game protocol
    ol-web/                  # axum/hyper site + API + static
    ol-persist/              # snapshots, WAL, mmap region files (NO SQL)
    ol-metrics/              # histograms, counters, tracing spans
    ol-server/               # binary: wire everything
  content/                   # symlink or path to OneLifeData7
  web/static/                # site assets
  docs/                      # this plan + research outputs
```

### 5.2 Core technology choices (recommended)

| Concern | Choice | Why |
|---------|--------|-----|
| Async I/O | **tokio** | mature, multi-thread runtime for net + web |
| HTTP | **axum** + tower | clean routing, static files, SSE/WebSocket later |
| Sim parallelism | dedicated OS threads or tokio **blocking** pool for pure sim | keep sim deterministic pacing |
| AI parallel | **rayon** or custom worker pool | CPU-bound planners |
| Serialization | custom binary + **rkyv** or **bincode** for snapshots | speed; schema versioned |
| Maps | memory-mapped region files (`memmap2`) | huge worlds, demand-paged |
| Logging | **tracing** + **tracing-subscriber** | structured, AI-readable |
| Metrics | **metrics** + Prometheus text exporter *or* embedded JSON | for evolution loops |
| Config | **toml** + hot-reload watch | like ServerSettings.txt |
| Pathfinding | hierarchical A* / JPS on local grids | scale with map size |

Avoid: full actor framework complexity early; avoid DB ORMs.

### 5.3 Simulation model

**Fixed timestep** (keep 20 Hz semantic `tick_dt = 0.05s` for game balance):

```
for each tick:
  1. drain IntentQueue (cap N commands / tick for fairness)
  2. apply movement integration
  3. player vitals (food, heal, exhaustion, temp)
  4. animals / time transitions on active chunks
  5. season / biome spread budgeted work
  6. produce net deltas (PU, MX, FX, …) → conn queues
  7. publish metrics + optional snapshot revision
```

**Determinism hooks:** seedable RNG streams per system (`rng_world`, `rng_ai`, `rng_combat`) so replays and AI experiments can bisect.

### 5.4 Protocol layer

- Keep OHOL framing: ASCII messages terminated by `#`.  
- `ol-protocol` is pure and unit-tested with fixtures from live captures.  
- Version negotiation via existing `SN` / `LOGIN` flow.  
- Feature flag for OpenLife object ID remapping (`VanillaObjIdMap` logic).

### 5.5 ECS-ish entity layout (players)

Prefer **SoA components** over one giant struct:

```
EntityId
Transform { x, y, exact, moving }
Body { age, food, food_max, heat, exhaust, wounds… }
Social { mother, father, leader, exiles, class }
Economy { coins, prestige, reputation }
Inventory { held, clothing[slots], belly }
NetLink { conn_id | AiControl }
LineageId
```

AI and net address entities by `EntityId`; sim owns components.

---

## 6. Large dynamic worlds without a DB

### 6.1 Why flat arrays fail

500×500 = 250k tiles. At 4k×4k = 16M tiles; with object helpers and multi-layer state, full residency becomes RAM- and scan-hostile. World time currently walks map fractions every tick — cost grows with area.

### 6.2 Chunked / streamed world

```
World
  ├─ ChunkCoord (cx, cy)  e.g. 64×64 tiles
  ├─ Chunk {
  │     biome: [u8; N],
  │     floor: [u16; N],
  │     obj: compact column or sparse map,
  │     helpers: slot arena,
  │     hidden_layer,
  │     heat_field,
  │     dirty flags,
  │     last_sim_tick,
  │  }
  ├─ ChunkIndex (hashmap / grid)
  └─ Resident set policy
```

#### Simulation policy (dynamic but scalable)

| Chunk state | Simulated? |
|-------------|------------|
| **Hot** — has players / AI / recent edits | Full every tick |
| **Warm** — near hot, or animals active | Reduced rate |
| **Cold** — wilderness far away | Time-skip formula OR lazy catch-up on load |
| **On disk** | Not in RAM; mmap or load on approach |

**Important:** “Vastly bigger” does **not** require every tile ticking at 20 Hz. Preserve *illusion* of continuous world via:

- Deterministic **catch-up** when a chunk wakes (`elapsed = now - last_sim_tick`)  
- Budgeted global systems (season snow spread) as **work queues**, not full scans  
- Sparse indices for special objects (ovens, graves, roads, banana plants) — already a pattern in Haxe `WorldMap`

### 6.3 Persistence without a database

| Layer | Mechanism | Purpose |
|-------|-----------|---------|
| **Content** | read-only files from OneLifeData7 | objects/transitions |
| **Chunk store** | one file per chunk or pack file + index | world durability |
| **WAL** | append-only command/event log | crash safety |
| **Snapshots** | periodic full or region snapshot | fast boot |
| **Lineage** | append-only binary or length-prefixed records + id index | character pages |
| **Accounts** | small mmap hash file or jsonl compacted offline | prestige/coins |
| **Metrics** | ring buffers + rotated files | evolution / ops |

**No SQLite required.** Optional later: import lineage into SQLite *offline* for analytics — not runtime dependency.

### 6.4 Write path (crash-safe)

```
intent applied on sim
  → mutate chunk (dirty=1)
  → append WAL record (optional batch)
  → every T ticks: flush dirty chunks to disk (background)
  → every B hours: backup generation (keep last K)
```

Mirror existing UX: reconnect after ~30s restart with player data intact.

### 6.5 Map generation

Keep PNG (or multi-tile atlas) as **authoring** format, but bake to chunk packs offline:

```
tools/bake-map --png mysteraV1.png --out world_pack/
```

Support multi-map stitching for “vast” worlds (island continents) while remaining toroidal or hard-bordered per config.

---

## 7. AI NPCs / players on the server

### 7.1 Split: body vs mind

| Layer | Runs where | Responsibility |
|-------|------------|----------------|
| **Body** | sim | same rules as human player (food, combat, USE validity) |
| **Mind (rule AI)** | AI worker pool | observe → goals → path → intents |
| **Mind (LLM)** | async tasks | dialogue, high-level job priority; rate-limited |

This matches the spirit of `PlayerInterface` / `WorldInterface` but enforces **no direct mutation**.

### 7.2 Rule AI port strategy

Do **not** port `AiBase.hx` (7k lines) as one blob.

1. Extract **behavior modules**: Survive, FeedOther, Flee, Hunt, Craft, Profession(Job), Follow, Combat, Temperature.  
2. Priority stack with cooldowns (data-driven TOML later).  
3. Crafting: precomputed reverse graph from transitions (already in TransitionImporter craft steps).  
4. Pathfinding: reusable crate; hierarchical for large maps.  
5. Profession jobs from TODO (`BASICFARMER`, `SMITH`, …) as strategy objects.

### 7.3 Parallel AI execution

```
each AI tick (e.g. 2–10 Hz per NPC, staggered):
  worker:
    obs = Observation::capture(entity, radius)  // read-only view
    plan = brain.think(obs)                     // pure-ish
    enqueue(plan.intents)
  sim:
    apply intents with same validators as humans
```

Stagger NPCs across frames so 200–2000 AIs do not think on the same tick.

### 7.4 LLM AI (optional feature)

Port concepts from `AiHandler` / `AIProvider`:

- Hourly call budget, mutex → **token bucket** (atomic)  
- Conversation log files (already in `log/ai_conversation_log_*.txt`)  
- Metrics: `llm_latency_ms`, `llm_calls_remaining`, `llm_errors`  
- Never block sim on HTTP to model provider

---

## 8. Web interface and lineage

### 8.1 Goals

Serve the **site itself** (landing from `WebServer/*.html`) plus richer data views inspired by:

`http://lineage.onehouronelife.com/server.php?action=character_page&id=...`

(Reference OHOL lineage: character identity, parents, children, death, age, generation, family name.)

### 8.2 Routes (v1)

| Route | Purpose |
|-------|---------|
| `GET /` | Landing + live counts (humans, AIs, season, starving) |
| `GET /stats/players` | Living table |
| `GET /stats/food` | Global food balance |
| `GET /stats/accounts` | Prestige leaderboard |
| `GET /stats/lineage` | Death reasons / age histograms |
| `GET /character/{id}` | **Family tree / character page** |
| `GET /family/{eve_id}` | Dynasty overview |
| `GET /api/metrics` | JSON/Prometheus metrics |
| `GET /api/snapshot` | Machine-readable server state summary |
| `GET /static/*` | assets |

### 8.3 Character page content (target)

- Name, age, birth/death time, death reason  
- Mother / father links, Eve/dynasty, generation  
- Children list (from reverse index)  
- Prestige, coins, class, last words  
- Simple HTML/SVG **ancestor + descendant tree**  
- Optional: cause-of-death aggregation for dynasty  

### 8.4 No-DB lineage indexing

```
lineage/
  records.wal          # append LineageRecord
  by_id.idx            # id → file offset (rebuildable)
  by_eve.idx           # eve_id → ids
  children.idx         # parent_id → child ids
  snapshots/           # periodic compacted view
```

Web handlers use **mmap + index**; rebuild index on startup if missing. Sim only appends records on birth/death (cheap).

### 8.5 Live data without blocking sim

- Background task every 1s: build `PublicSnapshot` from sim via channel request or double-buffer.  
- HTTP serves last snapshot.  
- Character pages mostly historical (indexes), not live locks.

---

## 9. Metrics, observability, AI-evolvable design

The server should be a **lab instrument**, not a black box.

### 9.1 Metric categories

| Category | Examples |
|----------|----------|
| **Latency** | `cmd_apply_seconds`, `tick_seconds`, `ai_think_seconds`, `net_rtt_hint`, `http_request_seconds` |
| **Throughput** | `intents_applied`, `ticks`, `bytes_out`, `chunk_loads` |
| **Saturation** | `intent_queue_depth`, `ai_queue_depth`, `hot_chunks`, `skip_ticks` |
| **Gameplay** | population, births/deaths, season, food %, prestige dist, combat events |
| **AI** | profession counts, craft fail rate, path fail rate, LLM budget |
| **Persistence** | wal_lag, flush_seconds, dirty_chunks |

### 9.2 Event stream (for evolution)

Append structured events (JSON lines or binary):

```json
{"ts":...,"kind":"death","id":121,"reason":"hunger","age":32.1,"eve":5}
{"ts":...,"kind":"metric","name":"tick_seconds","p99":0.012}
```

Offline tools / Grok can:

1. Detect regressions (p99 tick ↑)  
2. Propose config or code changes  
3. Run integration tests  
4. Compare event streams before/after  

### 9.3 Design for AI agents working on the code

- Small crates, clear interfaces  
- Property tests for transitions  
- Golden protocol fixtures  
- `docs/research/*` and this plan as ground truth  
- Feature flags for incomplete systems  
- `AGENTS.md` / project rules: coding standards + “never block sim on I/O”

### 9.4 Health endpoint

`GET /health` → `{ "tick": ..., "tps": 20.0, "skip_ticks": 0, "hot_chunks": 42, "ok": true }`

---

## 10. Implementation phases (PR-sized slices)

### Phase A — Skeleton (week-scale)

- [ ] Cargo workspace + `ol-server` binary  
- [ ] Config TOML  
- [ ] `ol-protocol` parse/serialize + tests  
- [ ] Tokio TCP accept + LOGIN challenge stub  
- [ ] `ol-metrics` + tracing  
- [ ] `ol-web` static landing + `/health`  

### Phase B — Content + empty world

- [ ] Load ObjectData + transitions  
- [ ] Chunk world + PNG bake tool  
- [ ] Spawn viewer: send map chunks to real OHOL client  
- [ ] Persistence: empty snapshot load/save  

### Phase C — Core verbs

- [ ] MOVE, USE, DROP, REMV, BABY basics  
- [ ] Transitions apply + MX updates  
- [ ] Food/age/death minimal  
- [ ] Intent queue architecture locked in  

### Phase D — Open Life rules (iterative)

- [ ] Temperature, seasons  
- [ ] Prestige/YUM/cravings  
- [ ] Combat core  
- [ ] Graves/curses  
- [ ] Coins/hire hooks  

### Phase E — AI

- [ ] Observation + path + eat/survive  
- [ ] Craft graph + MAKE command  
- [ ] Professions  
- [ ] LLM feature optional  

### Phase F — Lineage web

- [ ] Append-only lineage  
- [ ] Character page + family tree UI  
- [ ] Stats dashboards  

### Phase G — Scale

- [ ] Region sharding  
- [ ] Cold chunk sleep / catch-up  
- [ ] AI worker pool tuning  
- [ ] Load tests (bots)

Each phase ends with: **compile**, **test**, **metrics baseline**, **short note in changelog**.

---

## 11. Grok Build usage budget plan

**Canonical instructions live in this repo:**

- `AGENTS.md` — always-on project rules  
- `docs/GROK_BUILD.md` — full session playbook  
- `docs/USAGE_BUDGET.md` — spend log  
- `.grok/rules/budget.md` — short rule file  

### 11.1 Principle

Super Grok / subscription allowance is **finite until refresh**. Stay **under 80% total**, and **pace linearly** so early days do not burn the quota.

### 11.2 Source of truth: slash command `/usage`

In Grok Build, **always** check subscription/credit usage with:

```
/usage
```

| Slash command | Meaning |
|---------------|---------|
| **`/usage`** | Credit / subscription usage & billing — **budget source of truth** |
| `/context` | Session context window only (not Super Grok quota) |
| `/session-info` | Model, turns, session stats |
| `/compact` | Shrink session context when fat |

**Do not guess remaining budget.** Run `/usage` at session start and before expensive work (subagents, multi-crate refactors). If already at today's soft ceiling, stop implementing.

### 11.3 Formula

```
Cap            = 0.80 × full allowance
soft_ceiling(t)= Cap × (t / D)     # t = day index, D = days until refresh
daily slice    = Cap / D
```

Example: refresh in **10 days**, Cap = 80%:

| Day | Cumulative soft max |
|-----|---------------------|
| 1 | 8% |
| 2 | 16% |
| 3 | 24% |
| … | … |
| 10 | 80% |

Optional: under-spend may roll modestly, but never exceed soft ceiling without user OK, and never plan past 80%.

### 11.4 Cost control

Prefer:

- Small, focused sessions (one crate / one goal)  
- Research notes in `docs/research/` instead of re-reading 5k–7k line Haxe files  
- Local `cargo test` / `cargo check` (zero subscription cost)  
- No image/video tools unless requested  

Avoid: parallel heavy implementers, drive-by refactors, pasting entire legacy modules.

### 11.5 Session tiers

| Tier | When | Intent |
|------|------|--------|
| **T0 Local** | cargo yourself | 0% subscription |
| **T1 Micro** | one file / API | small |
| **T2 Feature** | one vertical slice | medium |
| **T3 Review** | review only | small |
| **T4 Spike** | research spike | ≤ one daily slice |

### 11.6 Daily workflow

```
1. /usage                         → check real subscription budget
2. Compare to soft ceiling(t)
3. T0 cargo check (local)
4. T1/T2 single scoped Grok task
5. cargo test (local)
6. Log row in docs/USAGE_BUDGET.md
7. Stop if near soft ceiling
```

### 11.7 Session contract

See paste block in `docs/GROK_BUILD.md` (includes **Budget** section with `/usage`).

---

## 12. Risks and open decisions

| Risk | Mitigation |
|------|------------|
| Protocol incompat with popular clients (hetuw, etc.) | Capture fixtures; test against real client early |
| Incomplete transition edge cases | Golden tests from live server recordings |
| AI behavior regression | Behavior scores in metrics; compare event logs |
| Huge map disk thrash | Resident set limits; prefetch ring around players |
| Non-determinism | Seeded RNGs; avoid HashMap iteration order in logic |
| Scope explosion (TODO.MD is enormous) | Phases A–C vertical slice first |
| Global mutex habits return in Rust | Code review checklist; `Intent` only API |

### Open decisions (resolve during Phase A)

1. Single sim thread vs multi-shard sim v1? (**Recommend:** single sim thread + parallel AI first; shard world storage early, shard sim later.)  
2. Exact chunk size (32 vs 64)?  
3. WAL every command vs dirty-chunk flush only?  
4. Stay wire-compatible forever vs versioned protocol extensions?  
5. LLM provider interface (OpenAI-compatible HTTP)?  

---

## 13. Appendix — key source map

### 13.1 Server module sizes (approx. lines)

| File | ~Lines | Role |
|------|--------|------|
| `GlobalPlayerInstance.hx` | 5300+ | player logic god-object |
| `TimeHelper.hx` | 2200+ | tick, seasons, world time |
| `TransitionHelper.hx` | 1400+ | USE/transitions |
| `WorldMap.hx` | 1300+ | map + save |
| `Connection.hx` | 1100+ | net + login |
| `MoveHelper.hx` | 700+ | movement |
| `AiHandler.hx` | 560+ | LLM |
| `Lineage.hx` | 530+ | family / stats |
| `WebServer.hx` | 350+ | HTTP |
| `AiBase.hx` | 7300+ | NPC brain |
| `AiHelper.hx` | 1700+ | AI utilities |
| `ObjectData.hx` | 1300+ | object defs |
| `ServerSettings.hx` | large | all tunables |

### 13.2 Current defaults (important numbers)

| Setting | Value |
|---------|-------|
| Game port | 8005 |
| Web port | 8080 |
| Tick | 1/20 s |
| Map sample | 500×500 (`mysteraV1Test.png`) |
| WorldTimeParts | 25 |
| NumberOfAis | 40 (min 20) |
| Save every | 600 ticks (~30 s) |
| Backup | ~8 hours |
| Save dir | `SaveFiles/` |
| Web dir | `WebServer/` |

### 13.3 Persistence files (no DB already)

- `CurrentBiomesN.bin`, `CurrentFloorsN.bin`, `CurrentObjectsN.bin`, `CurrentObjectsHiddenN.bin`  
- `CurrentObjHelperN.bin`, `PlayersN.bin`, `PlayerAccountsN.bin`, `LineagesN.bin`  
- `lastDataNumber.txt` + backups `1..10/`  
- Food/object count text stats  

Rust should **learn from this design**, not require SQL.

### 13.4 Success criteria (MVP)

1. Official / common OHOL client connects, moves, picks berry, ages, dies.  
2. Server hosts website with live player list.  
3. Character page shows mother/father/children for a dead id.  
4. ≥ N rule AIs survive/eat without blocking humans.  
5. Metrics show tick time p99 and command latency.  
6. World pack supports ≥ 4× area with chunk streaming.  
7. Zero database processes required.

---

## Next immediate actions

1. Create `docs/research/` stubs and complete **R1 protocol-matrix** (highest leverage).  
2. Scaffold `openlife-rs` workspace (Phase A) under a new folder (do not destroy Haxe tree).  
3. Start **USAGE_BUDGET.md** with your real refresh date and day-1 micro session.  
4. Capture 1–2 minutes of real client traffic to `docs/research/fixtures/` for golden tests.

---

*Document version: 1.0 — generated as rebuild blueprint for Open Life Reborn Rust server.*

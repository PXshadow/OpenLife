# Rust Open Life server architecture

**Workspace:** `C:\OhOl\OpenLife\openlife\RustServer\`  
**Binary:** `ol-server` (`crates/ol-server`)  
**Design goals:** multi-thread net + single-writer sim, no SQL, fast restart, protocol-compatible OHOL client.

---

## 1. Crate graph

```
ol-server (bin)
  ├─ ol-sim            simulation + intents + live AI sticky / adapters
  ├─ ol-ai             AI façade (re-exports below)
  │    ├─ ol-ai-api         PlayerWrite/Read interfaces, FoodSearch (r=40)
  │    ├─ ol-ai-pathing     pure path-reach / blockedByAI maps
  │    ├─ ol-ai-helper      Goal + priority ladder
  │    ├─ ol-ai-crafting    craft graph / plan / value
  │    └─ ol-ai-professions pure profession SMs
  ├─ ol-main-ai        ThinkPlan / plan_hungry_food over interfaces
  ├─ ol-player-helper  shared pure food scoring / eat gates
  ├─ ol-net            TCP, login bootstrap, outbound hub, ticket
  ├─ ol-protocol       parse/format wire tags
  ├─ ol-world          World grid, generate, OLW1/v2, journal
  ├─ ol-content        objects/transitions/categories load
  ├─ ol-config         server.toml / env
  ├─ ol-web            HTTP viewer + APIs
  ├─ ol-metrics        counters / ops series hooks
  └─ (content/)        OneLifeData7 on disk
```

| Crate | Responsibility |
|-------|----------------|
| `ol-protocol` | Tag parse, `#` frames, PU/MX/MC/FM/PM formatters |
| `ol-net` | Accept, per-conn tasks, `OutboundHub`, ticket verify |
| `ol-content` | Parallel load of object/transition defs |
| `ol-world` | `World` tiles, biome gen, OLW save/load (v1/v2), journal |
| `ol-sim` | **Sole world writer**: `SimState`, `apply_intent`, ticks; live AI adapters |
| `ol-ai-api` | Write/read traits + best-food DTOs (humans and AI share writes via NetIntent) |
| `ol-ai-pathing` | Pure not-reachable / hostile / blockedByAI timers |
| `ol-ai-helper` | Goals + Haxe priority ladder sensors |
| `ol-ai-crafting` | Reverse craft graph + value scoring |
| `ol-ai-professions` | Pure profession state machines |
| `ol-ai` | Façade re-exports (`ol_ai::*` stable for server/sim) |
| `ol-main-ai` | High-level think plans over interfaces |
| `ol-player-helper` | Shared pure SearchBestFood / yum gates |
| `ol-server` | Glue: boot, tick loop, self-play, NPC scheduler, config |
| `ol-web` | `/viewer`, `/api/*`, lineage pages |
| `ol-config` | Typed config from `server.toml` |
| `ol-metrics` | Metrics helpers |

**AI split design:** `docs/design/OL_AI_SPLIT.md`

---

## 2. Runtime topology

```
main (ol-server)
  ├─ load config + content (rayon)
  ├─ load or generate world (OLW magic + version)
  ├─ build SimState + craft graph + arm decays
  ├─ spawn:
  │    ├─ TCP accept → conn tasks → mpsc NetIntent
  │    ├─ sim tick thread/async loop (~20 Hz wall or catch-up)
  │    ├─ optional NPC scheduler thread
  │    ├─ optional self-play agents
  │    └─ HTTP (ol-web)
  └─ shutdown: autosave world / lineages / accounts
```

**Ownership model**

- **One writer** for `SimState` + `World` mutation on the sim path.  
- Network tasks only produce `NetIntent` and consume outbound bytes.  
- No Haxe-style global player mutex; avoid lock-order bugs by design.

---

## 3. Core types (`ol-sim`)

| Type | Role |
|------|------|
| `SimState` | players, social books, animals, env, craft graph, metrics, … |
| `Player` | position, food, age, held, clothing, wounds, move path, … |
| `NetIntent` | Login / KeepAlive / Move / Use / Drop / Raw / Disconnected |
| `OutboundHub` | fan-out bytes by conn_id (in `ol-net`) |
| `ContentDb` | object + transition tables |
| `World` | tile arrays (`ol-world`) |

### Intent path

```
TCP bytes
  → ol_protocol::parse
  → NetIntent
  → ol_sim::apply_intent(state, outbound, intent)
       match Login | Move | Use | Drop | KeepAlive | Raw | Disconnected
  → outbound packets (PU, MX, MC, PS, …)
```

`NetIntent::Raw` carries many SAY-style commands and less-common tags until they graduate to typed intents.

### Tick path

```
tick_loop (ol-server)
  ├─ catch-up skip_ticks (Haxe-style behind wall clock)
  ├─ tick_move_paths
  ├─ tick_vitals_with_metrics
  ├─ tick_auto_decays
  ├─ tick_animals_dt
  ├─ environment / weather / snow / fire slices
  ├─ gestation / prestige refresh / autosave timers
  └─ emit vitals / hunger emotes / HX as needed
```

---

## 4. Module map (`ol-sim/src`)

Organized by concern. **Large orchestration stays in `lib.rs`** (`apply_intent`, `apply_use_at`, spawn, move); pure rules live in sibling modules.

### Movement & space

| Module | Role | Port status |
|--------|------|-------------|
| `move_path.rs` | Timed path + PM + gates | **DONE** (`timed_movement` default on; dual-shoe/age/force/jump/OpenDoors residuals elsewhere) |
| `pathfind.rs` | Grid pathfinding | PARTIAL |
| `map_chunk.rs` | MC 32×30 | DONE-ish |
| `math_wrap.rs` | Torus wrap | DONE pure |
| `chunk_tier.rs` | Interest tiers | PARTIAL |
| `move_notes.rs` | Speed notes | PARTIAL |

### Objects & craft

| Module | Role | Port status |
|--------|------|-------------|
| `craft_graph.rs` | Reverse recipe graph | PARTIAL |
| `craft_plan.rs` / `craft_value.rs` | Plan/value helpers | PARTIAL |
| `object_tags.rs` | Description tags | PURE |
| `tools.rs` / `item_value.rs` | Tools/value | PARTIAL |

### Vitals & life

| Module | Role | Port status |
|--------|------|-------------|
| `player.rs` | Player struct | PARTIAL |
| `age_curves.rs` / `age_stage.rs` | Age curves | PARTIAL |
| `food_fill.rs` / `yum.rs` / `drain_est.rs` | Food | PARTIAL |
| `fertility.rs` / `gestation_tick.rs` / `feed.rs` | Birth/nurse | PARTIAL |
| `birth_fitness.rs` | Mother selection | PARTIAL |
| `death_cause.rs` / `death_inherit.rs` / `death_log.rs` | Death tags + InheritCoins + log | PURE + wired in lib |

### Combat & crime

| Module | Role | Port status |
|--------|------|-------------|
| `combat.rs` / `weapons.rs` / `heal.rs` | Combat | PARTIAL |
| `crime.rs` / `permissions.rs` / `locks.rs` | Ownership | PARTIAL |
| `curse.rs` / `shove.rs` | Curse/shove | PARTIAL |
| `reputation.rs` / `prestige.rs` | Scores | PARTIAL / PURE |

### Social & meta

| Module | Role | Port status |
|--------|------|-------------|
| `social.rs` / `ally.rs` / `relations.rs` | Follow/exile/family | PARTIAL |
| `leadership.rs` / `naming.rs` | Leaders/names | PARTIAL |
| `speech.rs` / `mute.rs` / `mumble.rs` | Chat | PARTIAL → mute_delivery DONE |
| `economy.rs` / `treasury.rs` / `debt_book.rs` / `score.rs` | Economy | PARTIAL |
| `accounts.rs` / `account_persist.rs` / `lineage_persist.rs` | Persist | PARTIAL |
| `twins.rs` / `war.rs` / `posse.rs` / `poll.rs` | Meta | PARTIAL (twins registry + wait queue) |

### Environment

| Module | Role | Port status |
|--------|------|-------------|
| `environment.rs` / `weather.rs` / `snow.rs` / `fire.rs` | Climate | PARTIAL |
| `animals.rs` / `animal_move.rs` / `hunt.rs` | Animals | PARTIAL |
| `heat_ideal.rs` / `day_phase_names.rs` / `biome_colors.rs` | Queries | PARTIAL |
| `biomes_query.rs` | Bad biomes | PARTIAL |

### AI

| Module | Role | Port status |
|--------|------|-------------|
| `ai_goals.rs` / `professions.rs` | Goals | PARTIAL (thin vs AiBase) |
| `ol-server` `npc_ai.rs` / `npc_activity.rs` / `selfplay.rs` | Drivers | PARTIAL |

---

## 5. Persistence (Rust)

| Format | File (typical) | Content |
|--------|----------------|---------|
| OLW (magic `OLW1`) | `SaveFiles/world*.bin` | bulk world + rotation `.bak.N` |
| OLW **version 1** | same | biomes/floors/objects + helpers (flat contained only) |
| OLW **version 2** | same (readable) | v1 + `creation_time`/`time_to_change` + one-level **nested** ids |
| OLW **version 3** | same (current write) | recursive `NestedHelper` meta (uses/owners/custom multi-level) + living_owners + ground_id (**NESTED-OLW1**) |
| Journal | world journal | append tile changes |
| OLN1 | `lineages_v1.bin` | lineages |
| OLA1 | `accounts_v1.bin` | soft accounts |
| Nested containers | **persisted in v3** | full recursive NestedHelper; wire still one colon level |
| `ground_id` | **OLW3** | map helpers; clothing/held still open |

---

## 6. Net / protocol crates

```
ol-net
  login_bootstrap.rs   SN, map, player list, lineages
  outbound.rs          OutboundHub close + send
  ticket.rs            OHOL ticket verify (toggle)

ol-protocol
  tags.rs              tag constants
  wire_out.rs          format helpers
  lib.rs               parse_message, ClientTag-ish formatters
```

---

## 7. Design principles for new code

1. **Pure where possible** — pure functions + unit tests; wire in `apply_intent` / tick.  
2. **Small modules** — avoid growing `lib.rs` further when extracting is clear.  
3. **Document intentional deltas** — in `docs/port/TODO_PORT.md`.  
4. **Haxe comments** — link `// Haxe: File.fn` near non-obvious ports.  
5. **Simplicity** — prefer clear data flow over cleverness.  
6. **Efficiency** — no full-map O(n) per tick without slicing (Haxe already slices poorly in places; improve carefully).

---

## 8. Test & boot evidence

| Check | How |
|-------|-----|
| Unit | `cargo test -p ol-sim --lib` |
| World persist | `cargo test -p ol-world --lib` |
| Server | `cargo test -p ol-server` |
| Play | `cargo run -p ol-server --release` :8005 + `/viewer` |
| Usage budget | `openlife/Tools/usage-budget/fetch-usage.ps1` |

---

## 9. Cross-reference

For Haxe call graphs and file matrix, see sibling docs.  
For “is feature X done?”, start at [TODO_PORT.md](TODO_PORT.md) then [FILE_MATRIX.md](FILE_MATRIX.md).

# Open Life Rust server architecture

**Workspace:** `openlife/RustServer`  
**Binary:** `ol-server` (`crates/ol-server`)  
**Date:** 2026-08-30  

This is the living map of how the Rust server is **supposed** to be structured (compile units, Haxe deltas, concern split) and **what is done vs still missing**. Daily port leftovers live in [`port/QUEUE.md`](port/QUEUE.md) and [`port/TODO_PORT.md`](port/TODO_PORT.md).

---

## 1. Goals

1. **Same game logic as Haxe** — tick order, protocol, professions, settings knobs that affect play. Code may be restructured; semantics must not be dropped (intentional Haxe bugs stay unless documented).
2. **Independent compile units** — Rust crates so editing pathing or professions does not rebuild combat/USE.
3. **One write path** — humans and AI issue the same `NetIntent` (MOVE/USE/DROP/SAY). `ol-sim` is the **sole world writer**.
4. **Do not mix concerns** — temperature ≠ food eating ≠ player tick ≠ world map time.

---

## 2. Why this is not a 1:1 Haxe package copy

| Topic | Haxe | Rust | Why |
|-------|------|------|-----|
| Modules | Few huge files (`GlobalPlayerInstance`, `TimeHelper`, `AiBase`) | Many crates + modules by **concern** | rustc compiles **crates** independently; huge files punish incremental builds |
| Threading | Per-connection threads + global/player/world mutexes | Net tasks → `NetIntent` → **single writer** sim | Same serial gameplay; no lock-order bugs |
| AI writes | AiBase often mutates player/world | AI → `PlayerWriteInterface` → **same NetIntent as humans** | One USE/MOVE/DROP path |
| AI reads | Shared world under locks | `PlayerReadInterface` / views / food search (r=40) | Fast AI without holding the write lock |
| Settings | `ServerSettings.txt` + `**default**` skip + re-read every 200 ticks | `server.toml` omitted keys = compiled defaults; re-read if **mtime** changed (~200 ticks) | Same idea; no Haxe RTTI |
| Mutex knobs | `UseOneGlobalMutex`, `UseOneSingleMutex`, … | **Not ported** (`SettingsHome::NotApplicable`) | Rust does not use that lock soup |
| Save | Legacy bins | OLW3 NestedHelper + OLN2/OLA1/… | Fast restart |
| Commands | Many bare words | `/` `!` `?` prefixes (documented exceptions) | Safer speech vs command |
| Web | `WebServer` in process | `ol-web` crate | Compile isolation |

**Keep from Haxe:** ~20 Hz tick + catch-up skip; protocol tags; profession SM / priority ladder; port-as-is when marked (e.g. quad-vs-linear auto-exile). Domestic dying factor is **applied** in Rust (Haxe line was unassigned).

---

## 3. Crate graph (compile units)

Workspace members (**25 crates**). rustc compiles **crates** independently; editing `ol-ai-pathing` must not rebuild USE/combat.

```text
ol-protocol, ol-metrics              leaves
ol-binary → ol-content               OLC1/OLT1 binary caches
ol-config                            server.toml + LiveSettings
ol-world → ol-content                grid / biomes / OLW3
ol-net → ol-protocol + ol-world      TCP + OutboundHub
ol-ai-api → ol-net                   write/read traits, FoodSearch r=40
ol-player-helper → ol-ai-api         shared food scoring / eat gates
ol-ai-pathing                        path-reach + PathfinderNew (std)
ol-ai-crafting → ol-content          craft graph / plan / value
ol-ai-helper → crafting + pathing + content
ol-ai-professions → helper + crafting + content
ol-ai                                façade (re-exports AI crates)
ol-main-ai → api + helper + player-helper
ol-move-rules → ol-content + ol-world   wrap / isClose / speed / jump (pure)
ol-transition-rules → ol-content        alt-outcome / fortification (pure)
ol-combat-rules → ol-world + ol-content angryTime / reputation / wound plans (pure)
ol-identity → ol-config                 AccountBook + OLA1
ol-social-rules                         speech range / doCommands parse (pure)
ol-food-eating → ol-identity            food-fill + food-store-max (pure)
ol-temperature                          heat ideal / clothing warmth (pure)
ol-sim → world + content + protocol + net + config
       + ol-ai* + player-helper + move-rules + rule crates + identity
                                 SOLE WORLD WRITER
ol-web → sim + world + content
ol-server → sim + net + web + config + main-ai
```

| Crate | Role | Status |
|-------|------|--------|
| `ol-protocol` | PU/MX/MC/FM/PM/PS parse/format | **Done** |
| `ol-net` | Accept, login, `OutboundHub`, ticket | **Done** |
| `ol-binary` | OLC1/OLT1 encode/parse | **Done** |
| `ol-content` | Objects, transitions, categories | **Done** (distance trailers done) |
| `ol-world` | Tiles, biome, NestedHelper, OLW3 | **Done** |
| `ol-config` | `server.toml`, LiveSettings, field map | **Done** + long-tail leftover knobs |
| `ol-ai-api` | `PlayerWrite/ReadInterface` | **Done** |
| `ol-ai-pathing` | Reach maps + PathfinderNew | **Done** (live Goto uses PathfinderNew) |
| `ol-ai-crafting` | Craft graph / plan / value | **Done** (live I/O still partial in sim) |
| `ol-ai-helper` | Goals + priority ladder | **Done** core |
| `ol-ai-professions` | Farm/smith/baker/potter/shepherd/cleanup SMs | **Done** pure; live adapters in sim |
| `ol-ai` / `ol-main-ai` / `ol-player-helper` | Façade / ThinkPlan / shared food | **Done** |
| `ol-move-rules` | Wrap, distance, isClose, jump, **calculateSpeed** pure, nest speed | **Done** core |
| `ol-transition-rules` | Alt-outcome / fortification (pure) | **Done** |
| `ol-combat-rules` | angryTime + reputation + wound plans (pure) | **Done** |
| `ol-identity` | AccountBook + OLA1 | **Done** |
| `ol-social-rules` | Speech range / doCommands parse | **Done** |
| `ol-food-eating` | Food-fill bands + food-store-max | **Done** |
| `ol-temperature` | Heat ideal / clothing warmth | **Done** |
| `ol-sim` | Intents, ticks, live adapters, **re-exports** of rule crates | **Done playable**; still the mega compile unit |
| `ol-web` | Viewer + stats | **Done** (lineage list/character/stats; no SQL) |
| `ol-server` | Boot, tick, NPC, LLM HTTP | **Done** glue |

**Hard rule:** AI and rule crates **never** mutate the world. They return plans/intents; `ol-sim` applies.

`ol-sim` keeps **thin re-exports** (`ai_goals.rs`, `craft_graph.rs`, `farmer_profession.rs`, `math_wrap.rs`, …) so old `crate::…` paths still compile. Edit the owning crate, not the re-export.

### 3.1 Haxe file → compile unit

Canonical map. File-level notes and chunk status: [`port/FILE_MATRIX.md`](port/FILE_MATRIX.md). One Haxe file often splits across a **pure crate** and an **`ol-sim` live adapter**.

#### `openlife/server/`

| Haxe file | Compile unit | Live adapter / notes |
|-----------|--------------|----------------------|
| `Server.hx` | **`ol-server`** (`main.rs`, `world_boot.rs`) | Boot + tick loop glue |
| `ThreadServer.hx` | **`ol-net`** | Tokio accept vs Haxe threads |
| `ServerTag.hx` | **`ol-protocol`** | VOG/PHOTO tags stubbed |
| `ServerHeader.hx` | — | includes only |
| `Connection.hx` | **`ol-net`** (login, hub, ticket) + **`ol-sim`** (MC/PU/MX send, `ai_takeover`) | Intentional split: net vs sim |
| `GlobalPlayerInstance.hx` | **`ol-sim`** (`player.rs`, `apply_intent`, `tick_vitals`, many modules) | Largest remaining mega-file; peel candidates below |
| `TransitionHelper.hx` | **`ol-sim`** (`use_transition`, `clothing_transitions`, `horse_mount`, `locks`, `alt_outcome`) | **Adapt:** peel to `ol-transition-rules` (not started) |
| `MoveHelper.hx` | **`ol-move-rules`** (pure speed/wrap/range/jump) + **`ol-sim`** (`move_path`, `move_live_gates`, `jump_bw`) | Pure peel **done**; bloody held override stays in sim |
| `TimeHelper.hx` | **`ol-sim`** (`tick_vitals`, `world_time`, `long_term`, `animals*`, `settings_live`, `nested_timers`, `fever_pe`, `angry_tick`) | Player tick ≠ world tick as **modules**, not crates |
| `WorldMap.hx` | **`ol-world`** (grid/persist) + **`ol-sim`** (`world_food_stats`, `object_counts_share`, `postload_wire`) | OLW3 NestedHelper |
| `TemperatureHandler.hx` | **`ol-sim`** `temperature_handler.rs` | **Adapt:** crate peel `ol-temperature` not started |
| `Biome.hx` | **`ol-world`** `biome.rs` + **`ol-sim`** `biomes_query` | |
| `Lineage.hx` | **`ol-sim`** `lineage_persist`, `prestige` | |
| `PlayerAccount.hx` | **`ol-sim`** `accounts`, `account_persist` | |
| `PlayerSoul.hx` | **`ol-sim`** `player_soul` | |
| `NamingHelper.hx` | **`ol-sim`** `naming.rs` | |
| `ScoreEntry.hx` | **`ol-sim`** `score_entry.rs` | |
| `WebServer.hx` | **`ol-web`** | Lineage product pages still open |
| `SerializeHelper.hx` | **`ol-world`** persist | |
| `ServerAi.hx` | **`ol-server`** `npc_ai.rs`, `selfplay.rs` | Thin vs AiBase |
| `AiHandler.hx` | **`ol-sim`** `ai_handler` (pure) + **`ol-server`** LLM drain | |
| `AIProvider.hx` | **`ol-server`** `ai_provider.rs` (HTTP) | Pure helpers in sim re-export |

#### `openlife/auto/`

| Haxe file | Compile unit | Live adapter / notes |
|-----------|--------------|----------------------|
| `AiBase.hx` | **`ol-ai-helper`** (ladder) + **`ol-ai-professions`** (SMs) | Live scan/USE in **`ol-sim`** `profession_scan` / **`ol-server`** `npc_ai` |
| `AiHelper.hx` | **`ol-player-helper`** (food) + **`ol-ai-pathing`** + **`ol-ai-crafting`** | Live scan `search_best_food_live` in sim |
| `Pathfinder.hx` / `PathfinderNew.hx` | **`ol-ai-pathing`** `pathfinder_new.rs` | Live Goto in sim |
| `PlayerInterface.hx` | **`ol-ai-api`** `PlayerWrite/ReadInterface` | Writes = `NetIntent` |
| `WorldInterface.hx` | **`ol-ai-api`** `WorldView` | |
| `Action.hx` + `actions/*` | **`ol-net`** `NetIntent` + **`ol-sim`** `apply_intent` | Not 1:1 action objects |
| `Ai.hx` / `AiPx.hx` | **`ol-main-ai`** `ThinkPlan` | Thin |
| `Say.hx` | **`ol-sim`** `speech` / `ai_say_helper` | |
| `Automation.hx`, `Overseer.hx`, `Interpreter.hx`, `BotType.hx`, `Role.hx`, `roles/*` | — | Unused / not ported (Haxe automation leftover) |

#### `openlife/settings/` + `openlife/data/`

| Haxe file | Compile unit | Notes |
|-----------|--------------|-------|
| `ServerSettings.hx` **knobs** | **`ol-config`** + **`ol-sim`** `settings_live` | Omit key = compiled default; hot-reload ~200 ticks |
| `ServerSettings.PatchObjectData` / `PatchTransitions` | **`ol-content` `src/patches/`** | **Not settings** — boot content rewrite; `apply_all_haxe_content_patches` |
| `Settings.hx` / `OpenLifeData.hx` | **`ol-config`** | Paths / data roots |
| `ObjectData.hx` | **`ol-content`** | |
| `ObjectHelper.hx` | **`ol-world`** NestedHelper + **`ol-sim`** `nested_body` | |
| `transition/TransitionData.hx` + `TransitionImporter.hx` | **`ol-content`** + **`ol-binary`** OLT1 | |
| `transition/Category.hx` | **`ol-content`** | |
| `object/player/PlayerInstance.hx` | **`ol-protocol`** PU + **`ol-sim`** `Player` | |
| `map/MapData.hx` | **`ol-world`** | |

### 3.2 What still needs adapting (split, not gameplay)

These are **compile-unit** gaps, not “game doesn’t run”:

| Need | Why |
|------|-----|
| **`ol-sim` still mega** | GPI apply + TimeHelper orchestration stay here **on purpose** (sole writer). Peel **pure** rules / identity / patches — see §5. |
| **`ol-transition-rules`** | Alt-outcome + horse formulas **done**. Clothing matrix / lockpick session still sim. |
| **`ol-combat-rules`** | angry + reputation + fever PE + **wound plans** **done**. HIT/wound **apply** still sim. |
| **`ol-social-rules`** | Speech parse **done**. Live SAY adapter still sim. |
| **`ol-food-eating` / `ol-temperature`** | Fill bands + **food-store-max** / heat-ideal **done**. `try_eat_held` still sim. |
| **`ol-sim` tests** | **`crates/ol-sim/src/lib_tests.rs`** — `cargo test -p ol-sim --lib`. Not compiled on `cargo check` / `cargo build`. |
| **Keep re-exports thin** | Do not copy profession/craft/path bodies back into sim. |
| **`Connection.hx` split is correct** | Do not merge net + sim; keep `NetIntent` as the boundary. |
| **TimeHelper split is correct** | Do not put eat/heat into `world_time.rs` or map decay into `tick_vitals`. |

---

## 4. Inside `ol-sim` — do not mix

Haxe `TimeHelper.DoTimeStuff` runs player and world work in one loop. Rust keeps **named homes**:

| Concern | Canonical file | Haxe | Must not contain |
|---------|----------------|------|------------------|
| **Temperature** | `temperature_handler.rs` (`map_temp_player` + `heat_ideal`) | `TemperatureHandler` + GPI `updateTemperature` | Eat, world decay |
| **Food eating** | `food_eating.rs` (`try_eat_held`); capacity math in **`ol-food-eating`** (`food_store_max`) | GPI `tryEat` / `doEating` | Temperature, world_time |
| **Player tick** | `player_tick.rs` (docs) + `tick_vitals` | `DoTimeStuffForPlayer` | Map-slice decay |
| **World tick** | `world_time.rs` / `long_term.rs` | `DoWorldMapTimeStuff` / long-term | try_eat, body heat |

Crate peels of temperature / food-fill / food-store-max are **done**. Live try_eat / tile heat apply stay in sim.

---

## 5. Adopt plan — crates vs modules (why / why not)

Haxe files are **not** 1:1 crates. rustc isolates **crates**; the sim must remain **one writer**. Below: adopt, do-not, and next steps.

### 5.1 GlobalPlayerInstance — **do not** make `ol-gpi` a second writer

**Why not a crate that owns USE/MOVE/tick:** Haxe `GlobalPlayerInstance` *is* the living player plus the methods that mutate the world. In Rust that work **is** `ol-sim` (`Player`, `apply_intent`, `tick_vitals`). A crate that applies those methods would either:

- mutate `World` / `SimState` (second writer — forbidden), or
- depend on `ol-sim` while `ol-sim` depends on it (cycle).

**Adopt instead:**

| Layer | Crate / module | Owns |
|-------|----------------|------|
| Data | future **`ol-player-types`** (optional) | `Player` fields, clothing slots, held nest types — no `SimState` |
| Apply | **`ol-sim`** | `apply_intent`, `tick_vitals`, USE/DROP/SAY |
| Pure rules | `ol-move-rules` (done); `ol-transition-rules` / `ol-combat-rules` / `ol-social-rules` (planned) | formulas only |

**Where to open:** [`crates/ol-sim/src/player/mod.rs`](../crates/ol-sim/src/player/mod.rs) — index of every player-related file. Body: `player/body.rs` (`Player` struct). Apply/tick stay in `ol-sim` (`tick_vitals` in `lib.rs`).

**Next steps:** peel **pure** transition/combat/social **formulas** first (same pattern as `ol-move-rules`). Do not move `tick_vitals` out of the writer crate. Optionally lift world decay/animals **out of** `tick_vitals` onto the outer tick loop (player step then world step).

### 5.2 TimeHelper — **do not** make one `ol-timehelper` crate

**Why not:** Haxe `DoTimeStuff` mixes player vitals and map-slice decay in one loop. A single TimeHelper crate would **undo** the split we already have (`player_tick` vs `world_time`). Orchestration that mutates `SimState` belongs in `ol-sim`.

**Adopt instead (already started as modules):**

| Haxe | Rust home | Crate peel? |
|------|-----------|-------------|
| `DoTimeStuffForPlayer` | `tick_vitals` + `player_tick.rs` (docs) | No — writer |
| `updateTemperature` | `temperature_handler.rs` | Later `ol-temperature` **pure** extras only |
| `tryEat` | `food_eating.rs` | Later `ol-food-eating` **pure** fill/yum |
| `DoWorldMapTimeStuff` / long-term | `world_time.rs` / `long_term.rs` | Later `ol-world-time` **pure** decay/respawn rolls |
| Animals | `animals` / `animal_move` / `animal_pop` | Stay sim (map writes) |

**Next steps:** extract remaining **pure** decay/respawn/angry formulas; keep `do_world_map_time_stuff` / `tick_vitals` in sim.

### 5.3 Lineage / PlayerAccount / Soul / ScoreEntry — **yes, planned `ol-identity`**

These are persist books + formulas, not the world writer. They **can** be a crate once types do not import `SimState`.

| Haxe | Today | Target |
|------|-------|--------|
| `Lineage.hx` | `ol-sim` `lineage_persist` + `prestige` | `ol-identity` OLN2 I/O + node types |
| `PlayerAccount.hx` | `accounts` / `account_persist` | `ol-identity` OLA1 + `AccountBook` |
| `ScoreEntry.hx` | `score_entry.rs` (SES1) | `ol-identity` prestige queue |
| `PlayerSoul.hx` | `player_soul` FIFO + `soul_live` | Pure FIFO in `ol-identity`; **live** `soul_view_for` stays sim |
| `NamingHelper.hx` | `naming.rs` | Can ride along or stay sim (needs player names) |

**Why not this week:** `score_entry` still calls `animal_move::is_bone_grave`; `lineage_persist` uses `social::LineageNode`; `soul_live` impls on `SimState`. Invert those deps first (pass grave-id / node types in).

**Next steps:** (1) `is_bone_grave` as a content/object helper, (2) `LineageNode` into identity types, (3) new crate with persist I/O, (4) sim adapters only.

### 5.4 AI vs world — **same write interface as humans; tighten reads**

**Already adopted:**

- Writes: `PlayerWriteInterface` → `NetIntent` (NPC `NpcWriteTx` in `ol-server`). No `World.write` in `npc_ai.rs`.
- Reads: `PlayerReadInterface` / `WorldView` / `PlayerView` / `FoodSearch` (`ol-ai-api`). Sim adapter: `ai_adapters.rs`.
- Policy: `ol-ai-helper`, `ol-ai-professions`, `ol-ai-pathing`, `ol-ai-crafting` never mutate the world.

**Still to adopt:**

| Gap | Plan |
|-----|------|
| Some NPC paths `try_send(NetIntent::…)` instead of `use_at` / `drop_at` / `say` | Route **all** writes through `PlayerWriteInterface` methods |
| Profession scan lives in `ol-sim` and takes world tiles | Keep scan **adapter** in sim; profession **SM** stays in `ol-ai-professions` |
| NPC thinks with `World` `RwLock` directly | Prefer `WorldView` / `PlayerReadHandles` at the edge |
| LLM HTTP in `ol-server` | Correct (I/O); prompts stay pure |

**Next steps:** wrap remaining `try_send` in npc_ai; do not add world mutation to AI crates.

### 5.5 Settings vs content patches — **own patch unit (done 2026-08-30)**

Haxe `PatchObjectData` / `PatchTransitions` are **not knobs**. They rewrite objects/transitions at boot. They must not live in `ol-config` / `server.toml`.

**Adopted:** `ol-content/src/patches/` — `apply_all_haxe_content_patches` after category expand + `changeToolTransitions`. Files: `object_data.inc.rs` (PatchObjectData), `ai_should_ignore` / `alt_outcome` / `hungry_work` / `transitions_horse` (PatchTransitions). Importer leftovers stay in `lib_tail.inc.rs`.

**Next steps:** new Haxe patch rows go in `patches/`, never in `GameplayKnobs`.

### 5.6 Peel order

1. Patches in `ol-content/src/patches` — **done**.  
2. `ol-transition-rules` (alt-outcome) — **done**.  
3. `ol-combat-rules` (angry + reputation) — **done**.  
4. `ol-identity` (AccountBook + OLA1) — **done**.  
5. `ol-social-rules` (speech parse) — **done**.  
6. `ol-food-eating` / `ol-temperature` — **done**.  
7. Named world step `tick_world_after_players` — **done** (still invoked from `tick_vitals` for API compat).  
8. Horse mount formulas + fever PE ladder + PrestigeClass + LineageNode + OLN map persist — **done** (2026-08-30).  
9. **Still in sim (live apply / remaining persist):** clothing matrix, lockpick session, HIT/wound apply, SES1 score-entry apply, soul FIFO live view. Clothing uses `Player`; lockpick uses process mutex; SES1 uses grave object ids. Writer tests: `src/lib_tests.rs`.

Live adapters and `SimState` **stay in `ol-sim`**.

---

## 5b. Target peels (status table)

Pure helpers only (no `SimState` mutation). Live adapters stay in `ol-sim`.

| Crate | Owns | Status |
|-------|------|--------|
| `ol-move-rules` | MoveHelper calcSpeed / isClose / jump / wrap | **Done** (bloody held override stays in sim) |
| `ol-content` `patches/` | Haxe PatchObjectData / PatchTransitions | **Done** (boot unit; not settings) |
| `ol-transition-rules` | Alt-outcome + horse mount/hitch formulas | **Done** (clothing matrix / lockpick session still sim) |
| `ol-combat-rules` | angryTime + reputation + fever PE + wound **plans** | **Done** (HIT/wound apply still sim) |
| `ol-identity` | AccountBook + OLA1 + PrestigeClass + LineageNode + OLN map I/O | **Done** (SES1 apply / soul live still sim) |
| `ol-social-rules` | Speech range / doCommands parse | **Done** (live SAY adapter in sim) |
| `ol-food-eating` / `ol-temperature` | Food-fill + food-store-max / heat-ideal | **Done** (try_eat / tile heat apply still sim) |
| World tick step | `tick_world_after_players` after player vitals | **Done** (`tick_vitals` still calls it so tests stay green) |
| `ol-gpi` / `ol-timehelper` as writers | — | **Rejected** (see §5.1–5.2) |

**Tests:** `crates/ol-sim/src/lib_tests.rs` (`cargo test -p ol-sim --lib`). `lib.rs` is apply/tick only (~0.5 MB); tests are a separate file not compiled on `cargo check`.

---

## 6. Settings

| Haxe | Rust |
|------|------|
| `**default** Name = value` skipped on read | **Omit the key** in `server.toml` → compiled Haxe default |
| Re-read every 200 ticks | Same cadence; skip parse if **mtime unchanged** |
| `Reflect.setField` onto statics | `apply_live_settings` onto `GameplayKnobs` / `SimState` |

**Boot + tick:** sim applies `server.toml` at start and polls `HotReloadTracker` on the tick loop.

**Never promote to Live** (Rust does not need them): mutex/socket/thread sleep flags, `Debug*`/`Trace*` (`RUST_LOG`), LLM API keys (env). Admin `Secret` is live (`!S`).

---

## 7. Status dashboard

### Playable / systems — largely done

| Domain | Status |
|--------|--------|
| Net + LOGIN + MC/PU/MX | **Done** |
| Timed MOVE + PM + nest/grave/VOG wrap | **Done** (residual: some `isClose` edges) |
| USE/DROP/REMV + clothing + containSize | **Done** core |
| Food/yum/eat + world food factor | **Done** core |
| Combat (bloody, wounds, fever, ally, coins, mosquito) | **Done** core |
| Birth/gestation/nurse/twins (same-server) | **Done** |
| World tick (season, animals, snow, nested timers) | **Done** core |
| Persist OLW3 / OLN2 / OLA1 / players / war / scores | **Done** |
| Settings hot-reload + large LiveSettings set | **Partial** — used knobs keep landing; mutex/debug skipped |
| Web viewer + food/accounts stats | **Partial** — full lineage product pages open |
| AI crates + ladder + profession SMs | **Core done** |
| PathfinderNew on live Goto | **Done** |
| LLM speech / say-helper / follow-walk | **Done** core |
| Horse hitch nest cargo (`apply_use_at` → `use_transition`) | **Done** |

### Split — done vs remaining

| Item | Status |
|------|--------|
| AI leaf crates + façade + `ol-sim` re-exports | **Done** |
| `ol-move-rules` peel | **Done** core |
| Temperature / food / player-tick **modules** | **Done** (not crates) |
| `ol-transition-rules` / `ol-combat-rules` / `ol-social-rules` | **Done** (formulas; live apply in sim) |
| Shrink `lib.rs` / move tests out | **Done** (`src/lib_tests.rs`) |

### Still missing / partial (highest first)

Canonical next-list: [`port/PRIORITY.md`](port/PRIORITY.md).

| Area | What’s left |
|------|-------------|
| **Settings** | ColdSeason / GrowNewPlantsFromExisting / MaxPlayersBeforeStartingAsChild **live**. Angry / DoorIds / AiIgnoredFloorIds / Secret/`!S` **live**. ~300 ModuleConst remainder accepted. |
| **Craft live** | staging + specials + countDone **live** |
| **isClose** | **DONE** vs live actions + protocol `KILL` killHelper (ally warn, exactTx, deadly float) |
| **TIME-LONG** | **DONE** live tick + nested multi-use / contained decay |
| **Web** | Lineage list + character + stats **live** (OHOL.com extras residual) |
| **Parity tests** | Residual ol-sim fails can reappear; treat as sensors vs Haxe |
| **AI live depth** | Core done; npc `fill_live_sensors` + escape live; YOU ARE gender-split; BowlFiller popcorn peer |
| **Parked** | PHOTO, VOG, multi-server twins |

### Non-goals

- PHOTO / VOG real backends  
- Multi-server twin sockets  
- SQL  
- Line-by-line Haxe style  
- Porting Haxe mutex/thread settings  

---

## 8. Dependency rules (no cycles)

```text
rules / AI crates → ol-content / ol-protocol / std (prefer)
                 ↛ ol-sim, ↛ ol-server

ol-sim → rules + world + content + ai*
ol-server → ol-sim (+ thin main-ai / llm HTTP)
```

---

## 9. Where to look

| Doc | Use |
|-----|-----|
| This file | Architecture + done/missing; **§5 adopt plan** (GPI / TimeHelper / identity / AI / patches) |
| [`port/ARCHITECTURE_RUST.md`](port/ARCHITECTURE_RUST.md) | Module-level Rust map |
| [`port/ARCHITECTURE_HAXE.md`](port/ARCHITECTURE_HAXE.md) | Legacy Haxe runtime |
| [`design/OL_AI_SPLIT.md`](design/OL_AI_SPLIT.md) | AI crate split |
| [`BUILD_SPEED.md`](BUILD_SPEED.md) | Incremental compile |
| [`port/TODO_PORT.md`](port/TODO_PORT.md) | File/chunk checklist |
| [`port/QUEUE.md`](port/QUEUE.md) | Next settings / AI fire |
| [`port/CHUNK_PROTOCOL.md`](port/CHUNK_PROTOCOL.md) | How to take one chunk |

**Loops (Grok schedulers):** **paused** 2026-08-30. Recreate 60s settings + AI (chain 2, grok-4.6, never `resume_from`) from `QUEUE.md` when resuming.

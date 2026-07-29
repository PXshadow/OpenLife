# Haxe Open Life server architecture

**Source root:** `C:\OhOl\OpenLife\openlife\`  
**Entry:** `server/Server.hx` → `main()` → content init → map load → `WebServer.Start()` → `TimeHelper.DoTimeLoop()`

---

## 1. Runtime topology

```
┌─────────────────────────────────────────────────────────────────┐
│ main thread                                                      │
│   Server.main()                                                  │
│     ObjectData + TransitionImporter init                         │
│     WorldMap generate | load                                     │
│     WebServer.Start()          ── HTTP stats (separate concern)  │
│     TimeHelper.DoTimeLoop()    ── ~20 Hz forever                 │
│       AiBase.StartAiThread()   ── AI decision thread             │
│       each tick:                                                 │
│         optional Server.Acquire() global mutex                   │
│         DoTimeStuff()                                            │
│         sleep to catch tickTime                                  │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│ ThreadServer accept loop                                         │
│   per client: Connection thread                                  │
│     parse ServerTag lines                                        │
│     TransitionHelper / GlobalPlayerInstance / MoveHelper         │
│     Connection.send* (ClientTag frames)                          │
└─────────────────────────────────────────────────────────────────┘
```

**Locks (legacy)**

| Mutex | Who holds | Protects |
|-------|-----------|----------|
| `Server` global | optional whole-tick | coarse serialization |
| `GlobalPlayerInstance` | player time + many actions | player list / player fields |
| `WorldMap` | map mutations | tile vectors / helpers |

Rust replaces these with a **single sim owner** (message-passing / one writer thread) + outbound hub.

---

## 2. Module ownership

| Module | Owns | Mutated by |
|--------|------|------------|
| `WorldMap` | biomes, floors, object ids, helpers, hidden/original layers, temps | TimeHelper world, TransitionHelper, animals, save |
| `GlobalPlayerInstance` | all living beings (human + AI) | Connection cmds, TimeHelper player, AI, combat |
| `Connection` | socket, send queue, login, map chunk interest | ThreadServer, send helpers |
| `Lineage` | family tree nodes | birth/death, leadership |
| `PlayerAccount` / `PlayerSoul` | soft identity / chat memory | login, death, AI chat |
| `TransitionImporter` | transition tables (read-mostly after boot) | boot only |
| `ObjectData` | object defs (read-mostly) | boot only |
| `AiBase` | AI state machine per AI player | AI thread |
| `ServerSettings` | knobs | boot + hot reload every ~200 ticks |

---

## 3. Boot call chain

```
Server.main
  ├─ ServerSettings.readFromFile
  ├─ ObjectData.DoAllTheObjectInititalisationStuff
  ├─ TransitionImporter.DoAllInititalisationStuff
  ├─ new WorldMap
  │    ├─ generate() | load from SaveDirectory
  │    └─ writeToDisk if new
  ├─ (load players / AI from save — Connection/WorldMap helpers)
  ├─ WebServer.Start
  └─ TimeHelper.DoTimeLoop
       ├─ DoTest
       ├─ AiBase.StartAiThread
       └─ loop: DoTimeStuff + sleep
```

**Rust map:** `ol-server::main` → `world_boot` → content load (`ol-content`) → `ol_world` OLW1 → `ol_sim::SimState` → net accept + tick loop.

---

## 4. Tick pipeline (`TimeHelper`)

### 4.1 Outer loop — `DoTimeLoop`

| Step | Function | Notes |
|------|----------|-------|
| Advance tick | `tick++` | skip extra tick if behind wall clock |
| Hot reload settings | every 200 ticks | `ServerSettings.readFromFile` |
| Body | `DoTimeStuff` | under optional global mutex |
| Pace | `Sys.sleep` | target `tickTime = 1/20` |

### 4.2 Per-tick body — `DoTimeStuff`

```
DoTimeStuff
  ├─ DoSeason(dt)
  ├─ for each human Connection: DoTimeStuffForPlayer
  │    + periodic sendToMeAllClosePlayers
  ├─ for each AI Connection: DoTimeStuffForPlayer
  ├─ DoWorldMapTimeStuff          // map slices, time transitions, water, uses
  ├─ RespawnObjects
  ├─ DoWorldLongTermTimeStuff     // decay, walls, snow, seasonal biomes
  └─ periodic map.writeToDisk / writeBackup / updateObjectCounts
```

### 4.3 Player slice — `DoTimeStuffForPlayer`

```
DoTimeStuffForPlayer
  ├─ DisplayStuff / DisplayClosePlayers
  ├─ UpdatePlayerStats
  ├─ DoLeadership
  ├─ DoTimeOnPlayerObjects        // held/clothing time transitions
  ├─ UpdateEmotes
  ├─ updateAge
  └─ updateFoodAndDoHealing
```

### 4.4 World slice — key functions

| Function | Role |
|----------|------|
| `DoWorldMapTimeStuff` | iterate map parts; tile temp; second outcomes; water move; time transitions |
| `doTimeTransition` / `doTimeTransitionHelper` | TIME transitions on helpers |
| `doAnimalMovement` | animals with time transitions |
| `DoAnimalDamage` | animal hurt players |
| `RespawnObjects` / `SpawnObject` | natural respawn |
| `DoWorldLongTermTimeStuff` | multi-year decay, floors, spring regrow, snow |
| `SpreadSnow` / `RemoveSnow` | winter overlay |
| `AlignWalls` | wall auto-alignment |
| `IsProtected` | decay protection near homes |

**Rust map:** `tick_vitals`, `tick_move_paths`, `tick_auto_decays`, `tick_animals`, environment/weather/snow/fire modules — **not 1:1 function names**; parity tracked in FILE_MATRIX / TODO_PORT.

---

## 5. Client command path

```
Connection thread reads line
  └─ parse ServerTag
       ├─ LOGIN / RLOGIN → Connection.login* → GlobalPlayerInstance spawn
       ├─ KA → keepAlive / MoveHelper position sync
       ├─ MOVE → player.move → MoveHelper
       ├─ USE/DROP/REMV/SREMV/SWAP/SELF/BABY/UBABY/KILL/...
       │     → TransitionHelper.doCommand → doCommandHelper
       │          → player.use / drop / swap / kill / ...
       ├─ SAY → player.say → commands + chat fanout
       ├─ EMOT / DIE / JUMP / FLIP / LEAD / GRAVE / OWNER / FORCE / PING
       └─ VOG* / PHOTO → admin / special (often restricted)
```

**TransitionHelper.doCommand** is the central gate for object interaction: moving/close checks, containers, clothing, horses, multi-use, numberOfUses.

---

## 6. File catalog (server package)

| File | ~LOC | Responsibility |
|------|------|----------------|
| `Server.hx` | 230 | main, global mutex, vanilla id map, map owner |
| `ThreadServer.hx` | 83 | TCP accept, spawn Connection threads |
| `ServerTag.hx` | 84 | inbound tag enum |
| `ServerHeader.hx` | 13 | includes |
| `Connection.hx` | 1079 | login, send*, close, map chunk, PU/MX helpers |
| `GlobalPlayerInstance.hx` | 5343 | **core game logic** (use/say/combat/birth/death/prestige…) |
| `TransitionHelper.hx` | 1444 | USE/DROP/REMV/SWAP pipeline |
| `MoveHelper.hx` | 705 | movement speed, path, force, PM |
| `TimeHelper.hx` | 2204 | 20 Hz tick + seasons + world sim |
| `WorldMap.hx` | 1288 | map storage, gen, save bins |
| `TemperatureHandler.hx` | 201 | tile/player temperature |
| `Biome.hx` | 157 | biome ids / passability colors |
| `Lineage.hx` | 530 | family tree |
| `PlayerAccount.hx` | 280 | soft accounts |
| `PlayerSoul.hx` | 477 | soul / chat memory |
| `NamingHelper.hx` | 337 | names |
| `ScoreEntry.hx` | 98 | scoreboard rows |
| `SerializeHelper.hx` | 61 | binary helpers |
| `WebServer.hx` | 350 | HTTP stats / pages |
| `ServerAi.hx` | 87 | AI player wrapper |
| `AiHandler.hx` | 568 | LLM AI provider path |
| `AIProvider.hx` | 148 | provider interface |

### AI package (`auto/`)

| File | ~LOC | Role |
|------|------|------|
| `AiBase.hx` | 7372 | professions, craft, combat AI — **largest port surface** |
| `AiHelper.hx` | 1768 | spatial search, food, heat, craft helpers |
| `Pathfinder.hx` / `PathfinderNew.hx` | 315 / 442 | pathfinding |
| `PlayerInterface.hx` / `WorldInterface.hx` | ~100 | AI abstraction over player/world |
| `Action.hx` + `actions/*` | small | action DSL experiments |
| `roles/*` | small | role examples |

### Settings

| File | ~LOC | Role |
|------|------|------|
| `ServerSettings.hx` | 3409 | all tunables |
| `Settings.hx` / `OpenLifeData.hx` | small | client/shared bits |

---

## 7. Data layer used by server

```
openlife/data/
  object/ObjectData.hx, ObjectHelper.hx     — defs + runtime instance
  object/player/PlayerInstance.hx           — wire PU fields
  transition/TransitionData.hx, TransitionImporter.hx
  map/*                                     — client map; server uses WorldMap primarily
```

Content on disk: `OneLifeData7/objects`, `transitions`, `categories`, `contentSettings`.

---

## 8. Persistence (Haxe)

| What | Where (typical) | Notes |
|------|-----------------|-------|
| World bins | `SaveFiles/` rotation | biomes, objects, floors, helpers |
| Players | with world / separate | reconnect → AI holds body |
| Lineages | binary | family |
| Accounts | binary | soft accounts |
| ServerSettings | `ServerSettings.txt` | hot reload |

**Rust:** OLW1 world, OLN1 lineages, OLA1 accounts, journal — see ARCHITECTURE_RUST.md.

---

## 9. Protocol tags (inbound ServerTag)

`KA USE BABY SELF UBABY REMV SREMV DROP SWAP KILL JUMP EMOT DIE GRAVE OWNER FORCE PING VOGS VOGN VOGP VOGM VOGI VOGT VOGX PHOTO SAY LOGIN RLOGIN MOVE FLIP LEAD`

Outbound: `openlife/client/ClientTag.hx` (PU, MX, MC, FM, PM, PS, …).

---

## 10. Product deltas vs vanilla OHOL

Documented in root `TODO.MD`: hard winter, strong animals in winter, temperature kill, prestige/yum, classes, fathers, grave curses, boats-as-cars, AI hire, etc. Port must preserve **Open Life Reborn rules**, not only vanilla Jason server.

---

## 11. AI lookup tips

- Start from **ServerTag** or **TimeHelper.DoTimeStuff** depending on net vs tick.  
- `GlobalPlayerInstance` is not modular: use **CALL_INDEX.md** section GPI.  
- Many Haxe `TODO`s are **known incompleteness** — port the *intended* behavior when clear; otherwise port *as-implemented* and note the TODO in HAXE_OPEN_TODOS.md.  
- Prefer reading **function bodies** over class headers; Haxe mixins / `using MoveHelper` matter.

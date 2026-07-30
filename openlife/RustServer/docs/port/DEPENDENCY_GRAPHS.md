# Dependency graphs (AI overview)

Mermaid graphs for human + AI navigation. Status colors are conceptual (see FILE_MATRIX).

---

## 1. Haxe runtime call graph (high level)

```mermaid
flowchart TB
  MAIN[Server.main] --> INIT[ObjectData + TransitionImporter]
  MAIN --> MAP[WorldMap generate/load]
  MAP --> POSTLOAD[ObjectHelper.InitObjectHelpersAfterRead]
  POSTLOAD --> ACCG[PlayerAccount.graves]
  POSTLOAD --> OWN[player.owning]
  POSTLOAD --> LIN[Lineage.ownsObject]
  MAIN --> WEB[WebServer.Start]
  MAIN --> LOOP[TimeHelper.DoTimeLoop]
  LOOP --> AI[AiBase.StartAiThread]
  LOOP --> TICK[TimeHelper.DoTimeStuff]
  TICK --> SEASON[DoSeason]
  TICK --> PLY[DoTimeStuffForPlayer]
  TICK --> WMAP[DoWorldMapTimeStuff]
  TICK --> RESP[RespawnObjects]
  TICK --> LONG[DoWorldLongTermTimeStuff]
  TICK --> SAVE[WorldMap.writeToDisk]
  SAVE --> FOODSTATS[writeFoodStatistics FoodStatsN.txt]
  SAVE --> OBJCOUNTS[TraceCountObjectsToDisk ObjectCountsN.txt]
  WEB --> FOODHTML[generateFoodStatistics HTML]

  ACC[ThreadServer.accept] --> CONN[Connection thread]
  CONN --> TAG{ServerTag}
  TAG --> LOGIN[login/rlogin]
  TAG --> MOVE[MoveHelper / player.move]
  TAG --> TH[TransitionHelper.doCommand]
  TAG --> SAY[player.say]
  TH --> USE[player.use / drop / swap / kill / baby]
  USE --> WM[WorldMap get/set helpers]
  PLY --> GPI[GlobalPlayerInstance fields]
  AI --> AIB[AiBase.doTimeStuff]
  AIB --> TH
  AIB --> MOVE
  SAY --> AIB
  AIB --> SAYH[AiBase.sayHelper]
  SAYH --> LLM[AiHandler.respondToPlayerAsync]
  SAYH --> SCRIPTED[HOLA/FOLLOW/DROP/GO HOME/HOME!]
  SCRIPTED --> MOVE
  SCRIPTED --> DROP[orderedToDrop / dropHeldObject]
```

---

## 2. Haxe package dependencies

```mermaid
flowchart LR
  subgraph server
    Server --> WorldMap
    Server --> TimeHelper
    Server --> Connection
    TimeHelper --> GlobalPlayerInstance
    TimeHelper --> WorldMap
    TimeHelper --> TemperatureHandler
    Connection --> GlobalPlayerInstance
    Connection --> TransitionHelper
    TransitionHelper --> GlobalPlayerInstance
    TransitionHelper --> WorldMap
    GlobalPlayerInstance --> Lineage
    GlobalPlayerInstance --> PlayerAccount
    GlobalPlayerInstance --> MoveHelper
    GlobalPlayerInstance --> NamingHelper
    ServerAi --> AiBase
    AiHandler --> AIProvider
  end
  subgraph auto
    AiBase --> AiHelper
    AiBase --> Pathfinder
    AiBase --> PlayerInterface
    AiHelper --> WorldInterface
  end
  subgraph data
    ObjectData
    ObjectHelper
    TransitionImporter
  end
  subgraph settings
    ServerSettings
  end
  server --> data
  server --> settings
  auto --> server
  auto --> data
```

---

## 3. Rust crate dependencies

```mermaid
flowchart TB
  SRV[ol-server] --> SIM[ol-sim]
  SRV --> NET[ol-net]
  SRV --> WEB[ol-web]
  SRV --> CFG[ol-config]
  SRV --> WLD[ol-world]
  SRV --> CNT[ol-content]
  SRV --> AI[ol-ai façade]
  SRV --> MAIN[ol-main-ai]
  SIM --> WLD
  SIM --> CNT
  SIM --> PROTO[ol-protocol]
  SIM --> NET
  SIM --> AI
  SIM --> PH[ol-player-helper]
  AI --> API[ol-ai-api]
  AI --> HELP[ol-ai-helper]
  AI --> PATH[ol-ai-pathing]
  AI --> CRAFT[ol-ai-crafting]
  AI --> PROF[ol-ai-professions]
  HELP --> PATH
  HELP --> CRAFT
  PROF --> HELP
  PROF --> CRAFT
  MAIN --> API
  MAIN --> HELP
  MAIN --> PH
  PH --> API
  NET --> PROTO
  WEB --> SIM
  WEB --> WLD
  SIM --> MET[ol-metrics]
```

See also `docs/design/OL_AI_SPLIT.md` for AI crate roles and dedupe status.

---

## 4. Rust sim intent + tick

```mermaid
flowchart LR
  TCP[TCP conn task] --> PARSE[ol-protocol parse]
  PARSE --> INTENT[NetIntent]
  INTENT --> APPLY[apply_intent]
  APPLY --> STATE[SimState + World]
  APPLY --> OUT[OutboundHub]

  BOOT[sim boot OLA1/OLW] --> POST[apply_init_object_helpers_after_read]
  POST --> STATE
  SPAWN[spawn_player] --> ROWN[rebuild_player_owning_from_world]
  ROWN --> STATE

  TICK[tick loop] --> MP[tick_move_paths]
  TICK --> VIT[tick_vitals]
  TICK --> DEC[tick_auto_decays]
  TICK --> ANI[tick_animals]
  TICK --> ENV[environment/weather/snow/fire]
  MP --> STATE
  VIT --> STATE
  DEC --> STATE
  ANI --> STATE
  ENV --> STATE
  STATE --> OUT
  STATE --> WFOOD[WorldFoodStats]
  WFOOD --> MIRROR[mirror_world_food_share]
  MIRROR --> AUTOSAVE[ol-server autosave]
  AUTOSAVE --> FSDUMP[write_food_statistics FoodStats.txt]
  APPLY -->|SAY human| FANSCRIPT[fan_out_ai_say_scripted]
  FANSCRIPT --> PLAN[plan_scripted_say_helper]
  FANSCRIPT --> GOTO[try_ai_follow_path_to MOVE/FOLLOW/GO HOME]
  FANSCRIPT --> LLM2[fan_out_ai_speech_llm skip handled]
  VIT --> ORD[tick_ordered_ai_drop]
  VIT --> FLW[tick_ai_follow_walk]
  ORD --> STATE
  FLW --> GOTO
```

---

## 4b. FOODSTATS-DISK (FoodStats + ObjectCounts residual)

```mermaid
flowchart TB
  subgraph Haxe
    WRITE[WorldMap.writeToDiskHelper] --> WFS[writeFoodStatistics]
    WFS --> FSFILE[FoodStatsN.txt]
    WRITE --> OCT[TraceCountObjectsToDisk]
    OCT --> OCFILE[ObjectCountsN.txt]
    WEB[WebServer.generateFoodStatistics] --> HTML[foodText HTML table]
  end
  subgraph Rust
    LIVE[SimState.world_food] --> SHARE[WorldFoodShare]
    SHARE --> SRV[ol-server autosave/shutdown]
    SRV --> WFR[write_food_statistics]
    WFR --> FSTXT[FoodStats.txt]
    SHARE --> WEBFOOD[ol-web food_view /stats/food]
    WEBFOOD --> HTMLOUT[format_food_statistics_html]
    LIVE --> FMT[format_stats_line / format_food_statistics_html]
    LT[LongTermState counts] --> OCPURE[write_object_counts pure]
    OCPURE --> OCTXT[ObjectCounts.txt path]
    CFG[ServerConfig food_stats_save_path / object_counts_save_path] --> SRV
    CFG --> OCPURE
  end
  WFS -.->|line shape| FMT
  WEB -.->|HTML shape| HTMLOUT
  OCT -.->|line shape| OCPURE
```

---

## 5. Haxe file → Rust crate mapping (coarse)

```mermaid
flowchart LR
  subgraph Haxe
    H_CONN[Connection.hx]
    H_GPI[GlobalPlayerInstance.hx]
    H_TH[TransitionHelper.hx]
    H_MH[MoveHelper.hx]
    H_TIME[TimeHelper.hx]
    H_WM[WorldMap.hx]
    H_AI[AiBase.hx]
    H_SET[ServerSettings.hx]
  end
  subgraph Rust
    R_NET[ol-net]
    R_SIM[ol-sim lib.rs + modules]
    R_WLD[ol-world]
    R_CNT[ol-content]
    R_CFG[ol-config / server.toml]
    R_NPC[ol-server npc_* selfplay]
  end
  H_CONN --> R_NET
  H_CONN --> R_SIM
  H_GPI --> R_SIM
  H_TH --> R_SIM
  H_MH --> R_SIM
  H_TIME --> R_SIM
  H_WM --> R_WLD
  H_WM --> R_SIM
  H_AI --> R_NPC
  H_AI --> R_SIM
  H_SET --> R_CFG
```

---

## 6. Transition / USE dependency (Haxe)

```mermaid
flowchart TB
  CMD[TransitionHelper.doCommand] --> H[doCommandHelper]
  H --> CLOSE[checkIfNotMovingAndCloseEnough]
  H --> DROP[drop]
  H --> USE[use]
  H --> SWAP[swap]
  USE --> CONT[doContainerStuff]
  USE --> HORSE[doHorseStuffPossible]
  USE --> TRANS[doTransitionIfPossible]
  TRANS --> NUM[DoChangeNumberOfUses*]
  TRANS --> SEND[sendUpdateToClient]
  DROP --> CLOTH[doPlaceObjInClothing]
  DROP --> CONT
```

**Rust analog:** `apply_intent` → `apply_use_at` / `apply_drop` / container helpers in `lib.rs` + modules. Gaps: full multi-use edge cases, horse/vehicle, clothing nested transitions — see TODO_PORT.

---

## 7. Tick player slice dependency (Haxe)

```mermaid
flowchart TB
  P[DoTimeStuffForPlayer] --> D[DisplayStuff]
  P --> S[UpdatePlayerStats]
  P --> L[DoLeadership]
  P --> O[DoTimeOnPlayerObjects]
  P --> E[UpdateEmotes]
  P --> A[updateAge]
  P --> F[updateFoodAndDoHealing]
  F --> TEMP[TemperatureHandler]
  A --> DEATH[doDeath paths]
```

---

## 8. AI dependency (Haxe)

```mermaid
flowchart TB
  THREAD[AiBase.StartAiThread] --> RUN[RunAi / doTimeStuff]
  RUN --> PRIO{priority}
  PRIO --> FLEE[escape combat/animals]
  PRIO --> EAT[SearchBestFood]
  PRIO --> FEED[feed baby/others]
  PRIO --> CRAFT[craftItem / GetOrCraft]
  PRIO --> JOB[profession jobs]
  PRIO --> FOLLOW[follow leader/player]
  CRAFT --> HELPER[AiHelper spatial]
  CRAFT --> PATH[Pathfinder*]
  CRAFT --> USE[myPlayer.use/drop/move]
```

**Rust:** `ai_goals` + self-play + NPC explore — **far thinner** than AiBase.

---

## 9. Fertility + twin-code wait (`FERTILITY-TWINS`)

```mermaid
flowchart TB
  LOGIN[LOGIN twin_code_hash twin_count] --> NET[ol-net Raw TWINJOIN]
  SAY[SAY TWINJOIN / ?TWINWAIT / ?TWINS] --> JOIN[apply_twin_join]
  NET --> JOIN
  JOIN --> Q[TwinWaitQueue]
  Q -->|Ready| PARTY[process_ready_twin_party]
  PARTY --> BABY[age-0 babies + lineage]
  BIRTH[SAY BIRTH / GESTATE] --> FERT[can_birth_full is_fertile]
  FERT --> FEM[player_is_female]
  NURSE[NURSE / HOLD feed / breastfeed tick] --> FERT2[player_is_fertile]
  MOTHER[pick_best_mother_p_id] --> FERT2
  DISC[Disconnected] --> LEAVE[twin_wait.leave]
  TICK[tick_vitals] --> DUE[due_mothers poll_due]
  DUE --> SPAWN[spawn_child]
```

Residual: multi-server `TwinRegistry` inter-server sockets/ping/handoff (**TWIN-MULTI-SERVER** PARTIAL pure+live registry); twin death heart-link; ObjectData.male content parse.

---

## 10. Session war / posse WPS1 (`SOCIAL-WAR-PERSIST`)

Haxe had **no** server disk for WAR/POSSE (protocol `WR`/`PJ` only). Rust product session maps:

```mermaid
flowchart TB
  BOOT[ol-server boot] --> LOAD[load_war_posse WPS1]
  LOAD --> SHARE[WarPosseShare Arc]
  SHARE --> SEED[sim seed apply_war_posse_snapshot]
  SAY[SAY WAR / POSSE / PEACE] --> MAPS[WarState + PosseState]
  DEATH[apply_death_inheritance] --> PRUNE[prune_war_posse_for_player]
  PRUNE --> MAPS
  MAPS --> DIRTY[war_posse_dirty]
  DIRTY --> MIRROR[mirror_war_posse_share]
  TICK[periodic tick mirror] --> MIRROR
  MIRROR --> SHARE
  SHARE --> SAVE[autosave / shutdown save_war_posse]
```

Residual: keys are session `p_id` (Players.bin sticky residual); disconnect without death keeps edges.

---

## 11. How an AI agent should use these graphs

1. Identify subsystem (net tag / tick phase / AI / persist).  
2. Open the matching graph section.  
3. Jump to **CALL_INDEX.md** for function names.  
4. Open Haxe file at function; open Rust module from **FILE_MATRIX.md**.  
5. Diff behavior; implement missing; update **TODO_PORT.md**.

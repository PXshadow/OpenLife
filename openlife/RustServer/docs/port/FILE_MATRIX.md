# File matrix: Haxe → Rust

**Legend:** DONE | PARTIAL | STUB | PURE | MISSING | NA  
**Update rule:** every port chunk must touch the relevant row(s).

Last reviewed: **2026-07-28** (AI-PROVIDER llm_http) (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)

> **Note (2026-07-29):** Full historical matrix was truncated mid-session; reconstructed from live tree + TODO_PORT / prior greps. Prefer **TODO_PORT.md** for exhaustive chunk status when rows conflict.

---

## A. `openlife/server/`

| ID | Haxe file | ~LOC | Rust targets | Status | Notes / missing highlights |
|----|-----------|------|--------------|--------|----------------------------|
| S-MAIN | `Server.hx` | 230 | `ol-server/main.rs`, `world_boot.rs` | PARTIAL | Boot order OK; vanilla id map partial; global mutex N/A by design |
| S-THREAD | `ThreadServer.hx` | 83 | `ol-net` accept | DONE | Tokio/async accept vs threads |
| S-TAG | `ServerTag.hx` | 84 | `ol-protocol`, `NetIntent` | PARTIAL | All tags recognized? VOG/PHOTO stubs |
| S-HDR | `ServerHeader.hx` | 13 | — | NA | includes only |
| S-CONN | `Connection.hx` | 1079 | `ol-net`, `login_bootstrap`, `outbound`, sim send helpers, **`ai_takeover`** | PARTIAL | Login/ticket/MC/PU/MX core; **close→AI-TAKEOVER** + rlogin reclaim wired; **FERTILITY-TWINS** twin wait queue core; **LEADER-RANGE** leader PU exempt + LEAD pin; **sendSayToAllClose** **PO-MAX-DISTANCE DONE** CloseForSay=20 (`ADULT_CHAT_RANGE`; mute/deaf Rust product; residual Euclidean vs Chebyshev) |
| S-GPI | `GlobalPlayerInstance.hx` | 5343 | `ol-sim` `lib.rs` + many modules | PARTIAL | Largest gap surface; see chunks in TODO_PORT; **MUTE-SAY** + **GPI-DEATH** + **COMBAT-BLOODY** + **ALLY-STRENGTH** + **PRESTIGE-ALLY-COST** + **REPUTATION-HIT** + **BREASTFEED-EDGES**; **updateTemperature** body heat + tile ambient **MAP-TEMP-PLAYER** |
| **TH-ALT-OUTCOME** / alt_transition_outcome | TransitionHelper alternativeTransitionOutcome + fortification | **DONE** (core) | `alt_outcome.rs` pure + ContentDb side-tables + wall/door push(0) + OLC1 `finish_cache_boot` + live `apply_use_at` TryAgain/Proceed; residual: LiveSettings knobs, coinCost, fortify-apply L190 |
| S-TH | `TransitionHelper.hx` | 1444 | `apply_use_at`, `apply_drop`, `multi_use`, `use_transition`, **`clothing_transitions`**, **`horse_mount`**, **`locks`**, **`alt_outcome`** | PARTIAL | Live USE=`use_transition`; held_uses; reverse/maxUse; minUseFraction; useChance; switch/force uses; loved-food polish; **TH-CLOTHING-MATRIX** core+gaps; **TH-HORSE + HORSE-MOUNT-POLISH**; **TH-LOCK + LOCKPICK-SETTINGS**; **IS-CLOSE** + **GPI-TOO-CLOSE**; **CLOTHING-CONTAIN-SIZE** + **USE containerIndex / L1087**; **TH-ALT-OUTCOME DONE (core)** pure+live+patches. Gaps: chest coin store; client OLC1 v8 bake; LiveSettings alt knobs / coinCost / fortify-apply |
| **JUMP-BW-FULL** | `MoveHelper` + `GPI.jump` + Server JUMP | — | `jump_bw` + `apply_player_jump` + MOVE gates | **DONE** (core) | pure plan + live JUMP/AI/MOVE; residual dropPlayer transform |
| S-MOVE | `MoveHelper.hx` | 705 | `move_path`, `move_speed`, `move_live_gates`, `move_notes`, `apply_move_*` | **DONE** (+ TIMED-MOVEMENT-DEFAULT + **IS-CLOSE** + **MOVE-MIDPATH** + **MOVE-VOG-WRAP** + **MOVE-NEST-SPEED** + **MOVE-GRAVE-ALL-PATHS**) | timed_movement default on; dual shoes; age/waitForForce/jump/OpenDoors/forceStop; **action_range**; mid-path recon; **VOG cancel** + **world-wrap CancleMovement**; **held nest + clothing backpack nest mult**; **grave curse live gates on all calculateSpeed paths** (path-start/finish/cancel); residual: flat↔nest backpack dual-write, Connection MaxDistance fans |
| S-TIME | `TimeHelper.hx` | 2204 | tick_* + environment + `world_time` + `animal_move` + `animal_damage` + **`animal_pop`** + `long_term` + settings hot-reload + `contained_timers_persist` + **`nested_timers`** + **`fever_pe`** | PARTIAL | **TIME-WORLD** + **TIME-LONG** + **damage_escape** + **chase_biome** + **pop_die_offspring** + settings reload + **CONTAINED-TIMERS-PERSIST** + **NESTED-IN-NESTED-TIMERS** + **FEVER-EMOTE** + **CURSED-GRAVES-INDEX** + **CURSED-GRAVE-TELEPORT** (`!TCG`/`!TV` consumers); residual AiHelper.SearchNewHome local oven; **COMBAT-MOSQUITO-KIND DONE** (live wire) |
| FEVER-EMOTE | `TimeHelper.UpdateEmotes` + `GPI.hasYellowFever` / `isSuperHot` / `doEmote` | — | `ol-sim/fever_pe.rs` + `tick_update_emotes` + feed ill PE | **DONE** (core) | pure ladder yellowFever(7)/heatStroke(21); tick%30; ambient 9s; feed-ill PE; residual: starve PE food&lt;0 vs hunger PE food&lt;3 index mismatch |
| S-WMAP | `WorldMap.hx` | 1288 | `ol-world` persist + postload + **`ol-sim/world_food_stats`** + **`object_counts_share`** + ObjectCounts pure | PARTIAL → postload + **WORLD-FOOD-FACTOR** + **FOODSTATS-DISK** + **EATEN-FOOD-PCT** + **OBJECTCOUNTS-LIVE** + **LINEAGE-24H** | OLW3 + live eaten% + FoodStats dump + **ObjectCounts census DONE**; **getStarvingFoodFactor** 24h DONE |
| S-TEMP | `TemperatureHandler.hx` | 201 | `environment`, `heat_ideal`, `world_time`, **`map_temp_player`** | **DONE** (player wire) | Map-slice + insulation; **BalanceTemperatureArea live**; residual closest-heat / clothing rValue matrix |
| S-BIOME | `Biome.hx` | 157 | `ol-world/biome`, `biomes_query` | PARTIAL | IDs; deep river / special biomes TODOs in Haxe too |
| S-LIN | `Lineage.hx` | 530 | `lineage_persist`, relations, **`prestige`**, **world_food_stats LINEAGE-24H** | PARTIAL → **CLASS-BONI + LINEAGE-24H DONE** | OLN2 deathTime/reason; PrestigeClass; **reasonKilledLastDay 24h** + boot seed stamps; archive/delete residual |
| S-ACC | `PlayerAccount.hx` | 280 | `accounts`, `account_persist`, `postload_wire` | PARTIAL | OLA1 + session graves + family_prestige + score_entries + **WEB-ACCOUNTS-STATS** female/male/is_ai snapshot |
| S-SOUL | `PlayerSoul.hx` | 477 | `player_soul` + `player_soul_wire` + `soul_live` + `Player.soul` | **PARTIAL → AI-SOUL-WIRE DONE** | Pure FIFO + prompts; sticky soul; residual ObjectData.male / LiveSettings caps |
| S-NAME | `NamingHelper.hx` | 337 | `naming.rs` | PARTIAL | Random names; family name file growth incomplete |
| S-SCORE | `ScoreEntry.hx` | 98 | `score.rs` + **`score_entry.rs`** | **DONE** (core) | SES1 prestige queue; residual LiveSettings mali factors |
| S-WEB | `WebServer.hx` | 350 | `ol-web` | PARTIAL | Viewer/APIs; **FOODSTATS-WEB** `/stats/food`; **WEB-ACCOUNTS-STATS** `/stats/accounts` DONE; lineage death-reason HTML (**WEB-LINEAGE-STATS**) |
| S-SER | `SerializeHelper.hx` | — | `ol-world` / nested persist | PARTIAL | NestedHelper OLW3 |
| S-AI | `ServerAi.hx` | — | selfplay / npc_ai | PARTIAL | Thin vs AiBase; **PATH-REACH-MERGE** npc pull-once/push + AI-TAKEOVER push |
| S-AIH | `AiHandler.hx` | — | `ai_handler` pure | **DONE** (pure) | LLM prompt pure; live wire stack separate |
| S-AIP | `AIProvider.hx` | — | `ai_provider` HTTP | **DONE** (HTTP) | MiniMax/Anthropic drain |

---

## B. `openlife/auto/`

| ID | Haxe file | Rust targets | Status | Notes |
|----|-----------|--------------|--------|-------|
| A-BASE | `AiBase.hx` | `ai_goals`, `priority_ladder`, profession modules, craft/job wires, **`ai_path_reach` dual maps** | PARTIAL | **AI-PRIO** + jobs/craft/path partial; **PATH-REACH-MERGE dual_map_merge DONE** (Player↔NPC pull/push + publish preserve + AI-TAKEOVER push); largest open surface |
| A-HELP | `AiHelper.hx` | pathfind, craft helpers, **`search_best_food`**, deadly scans | PARTIAL | **SEARCH-BEST-FOOD** + PATH-REACH + **PATH-REACH-MERGE DONE** + AI-ANIMAL-GOTO + AI-GOTO-FOOD core |
| A-AI | `Ai.hx` / `AiPx.hx` | thin | PARTIAL | |
| A-PF | `Pathfinder*.hx` | pathfind modules | PARTIAL | |
| A-ACT | `Action.hx` + `actions/*` | intent / action enums | PARTIAL | |
| A-REST | other auto modules | various | PARTIAL / NA | See TODO_PORT AI section |

---

## C. `openlife/settings/`

| ID | Haxe file | Rust targets | Status | Notes |
|----|-----------|--------------|--------|-------|
| C-SS | `ServerSettings.hx` | `ol-config` + `LiveSettings` + `GameplayKnobs` | PARTIAL | hot-reload + **C-SS-FULL-TABLE** + **C-SS-MORE** + **C-SS-TAIL-KNOBS** + **C-SS-AGE-FOOD** + **C-SS-MORE-KNOBS** + **C-SS-WOUND-HEAL** + **C-SS-MALE-HEAL** + **C-SS-MORE-BATCH3** + **C-SS-MORE-BATCH4** + **C-SS-MORE-BATCH5** + **C-SS-TEMP-HEAL** + **C-SS-MIN-AGE-AI DONE**; residual clothing MinAge commented (Haxe) + ~18 critical ModuleConst + ~170 Haxe long-tail; **C-SS-MORE-BATCH5 DONE** (weapon CD / jump exh / hungry USE heat pipe / AI speed / animal residual live CD) |
| C-SET | `Settings.hx` / `OpenLifeData.hx` | paths / data roots | PARTIAL | |

---

## D. High-value chunk ids (subset; see TODO_PORT for full)

| Chunk ID | About | Status | Notes |
|----------|-------|--------|-------|
| **FOODSTATS-DISK** / foodstats_txt | WorldMap.writeFoodStatistics FoodStats dump | **DONE** | `WorldFoodShare` + autosave/shutdown; HTML → **FOODSTATS-WEB** |
| **FOODSTATS-WEB** / food_stats_html | WebServer.generateFoodStatistics `/stats/food` | **DONE** | `WebState.food_view` = live WorldFoodShare; pure HTML Food/Eaten/Related |
| **OBJECTCOUNTS-LIVE** / object_counts_share | WorldMap TraceCountObjectsToDisk ObjectCounts dump | **DONE** | share + `count_objects_from_world` / `update_object_counts` nest census; boot seed; periodic recompute; autosave `ObjectCounts.txt` |
| **EATEN-FOOD-PCT** / world_food_map | live eaten% on add | **DONE** | |
| **WORLD-FOOD-FACTOR** | getFoodFactor / starving | **DONE** (core) | lineage 24h → **LINEAGE-24H DONE** |
| **LINEAGE-24H** / starving_window | reasonKilledLastDay 24h + getStarvingFoodFactor | **DONE** (core) | OLN2 death fields + boot seed; full stats pure maps + kill-name remap + HTML pure; residual ol-web table wire / ages birthTime for living |
| GPI-FOOD / CRAVING / SEARCH-BEST-FOOD / C-SS-FULL-TABLE | yum/meh + craving + search + FoodFactor Live | PARTIAL → most DONE | residual foodObjects reg order |
| GPI-DEATH / place_grave / PLACE-OBJECT | death/grave/place | **DONE** (core) | |
| **WALLET-COINS** / take_coins | takeCoins wallet on wound/damage | **DONE** | coins_stolen + economy gift; HIT lethal+equip; Player.dark_nosaj + CoinsOnWounding LiveSettings; residual i32 wallet floor |
| **DARK-NOSAJ** / dark_nosaj_use | Tarr 3112 + Dark Nosaj 2466 USE set/clear | **DONE** | pure `plan_monument_use` + damage×1.2 + eat prestige gate; Player.praised_jinbali; single USE side-effects + CU/say; residual i32 wallet floor (WALLET); sendFoodUpdate FX optional |
| PLAYERS-BIN / NESTED-* / SOCIAL-WAR-PERSIST | persist shares | **DONE** (core) | |
| LEADER-RANGE / FOLLOW-HIRE / MAP-LOCATION-PINS / PO-FAR | social UX | **DONE** (core) | |
| **PO-MAX-DISTANCE** / close_say_range | MaxDistanceToBeConsideredAsCloseForSay=20 | **DONE** | `ADULT_CHAT_RANGE`≠`NEARBY_RANGE`; pending-follower spoken_says + social-pin/coins/moskitos say fans; ModuleConst residual |
| **CURSED-GRAVES-INDEX** / cursed_graves | ClearCursedGraves + ovens + GetClosestBoneGrave + CursedGraveTime | **DONE** (core) | `WorldMapTimeState.cursed_graves`/`ovens`; map-slice fill + step%2000 prune; wolf global closest; sharp-stone +43200s; residual SearchNewHome local oven + CursedGraveTime LiveSettings |
| **CURSED-GRAVE-TELEPORT** / tcg_tv_teleport | GPI `!TCG`/`!CURSEDGRAVE` + `!TV`/`!VILLAGE` + teleport/doTeleport | **DONE** (core) | pure `teleport_cmd` + `Player.blocked_teleport_locations` + live `apply_do_teleport`/`try_apply_teleport_bang` SAY wire; godmode stands in for canUseServerCommands; residual torus-wrap distance + map_linear_index y−1 delta |
| COMBAT-BLOODY / WEAPON-* / ALLY / PRESTIGE-ALLY / REPUTATION-HIT / FEVER-BLEED / **WALLET-COINS** | combat | **DONE** (core) | residuals in TODO_PORT |
| **COMBAT-MOSQUITO-KIND** / mosquito_animal | Mosquito Swarm 2156 AnimalKind + biomeLove(JUNGLE) | **DONE** (core) | `AnimalKind::Mosquito` path damage=1; pure jungle love; live `player_jungle_biome_love` + path moskito scale + spawn×2 + chase/escape `is_deadly_animal`; residual content swamp/mapChance + BiomeAnimalHitChance jungle-escape |
| TIME-ANIMAL-* / TIME-LONG / MAP-TEMP-PLAYER | world tick | **DONE** (core) | |
| **MOVE-VOG-WRAP** / cancel_wrap | VOG on CancleMovement + world-wrap fold | **DONE** | `fold_relative_around_world` / `fold_world_pos_around_world` / `cancel_movement(use_vog)` + always MC; wrap gate on path start; jump/blocked `quad>25` VOG |
| **C-SS-AGE-FOOD** / age_food_max | NewBorn/OldAge FoodStoreMax Live | **DONE** | LiveSettings + GameplayKnobs + `FoodStoreMaxKnobs` pure bands; spawn/vitals/eat wire; residual combat `apply_damage_food_pipe` module defaults |
| AI-PRIO / AI-JOB-* / CRAFT-LIVE / AI-LLM-* | AI | PARTIAL | **AI-CRAFT-LIVE-RESID** + **AI-CRAFT-NPC-ENQUEUE** DONE (pile_id/full_piles/num_slots/peer blockedByAI); **AI-JOB-SMITH-RESID core** (PlayerSnapshot home/last + multi-prof npc peer + chisel load cache); multi residual (GetCraftAndDrop/WaterSource); see TODO_PORT |
| **AI-JOB-SMITH-RESID** / smith_chisel_resid | `AiBase` smith + `PatchObjectData` Chisel | **PARTIAL→core** | `PlayerSnapshot.home_*`/`is_last_*`; `NpcProfessionPeerRow`/`npc_peer_count_for_kind`; `SteelChiselFamilyTable`; wound peer skip; residual live USE I/O polish |

---

## E. OBJECTCOUNTS-LIVE detail

| Surface | Path | Role |
|---------|------|------|
| Pure format/write | `ol-sim/long_term.rs` | Haxe line shape + disk write |
| Pure census | `long_term::count_objects_from_world` / `update_object_counts` | ground + contained nest (Haxe `countObjects`) |
| Period | `should_update_object_counts` / `OBJECT_COUNTS_RECOMPUTE_TICKS` | Haxe `(tick+20)%TicksBetweenSaving` |
| Share types | `ol-sim/object_counts_share.rs` | `ObjectCountsSnapshot` / `ObjectCountsShare` |
| Sim mirror | `ol-sim/lib.rs` `mirror_object_counts_share` | periodic + disconnect (+ boot seed) |
| Boot | `SimBootLive.object_counts_share` + `ensure_counts_for_dump` | outer Arc + non-empty early dump |
| Config | `ol-config` `object_counts_save_path` | `ObjectCounts.txt` |
| Server I/O | `ol-server/main.rs` | autosave 60s/SAY SAVE + shutdown |
| Tests | `object_counts_share::*` + `long_term::count_objects*` / `format_object_count*` / `update_object_counts*` | pure |

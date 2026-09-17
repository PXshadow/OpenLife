# File matrix: Haxe → Rust

**Legend:** DONE | PARTIAL | STUB | PURE | MISSING | NA  
**Update rule:** every port chunk must touch the relevant row(s).

Last reviewed: **2026-07-28** (AI-PROVIDER llm_http) (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)

**Crate ownership (Haxe file → compile unit):** [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md) §3.1 — that table is canonical for split. This matrix is **file/chunk status**. Prefer **TODO_PORT.md** when rows conflict.

> **Note (2026-07-29):** Full historical matrix was truncated mid-session; reconstructed from live tree + TODO_PORT / prior greps.

---

## A. `openlife/server/`

| ID | Haxe file | ~LOC | Rust targets | Status | Notes / missing highlights |
|----|-----------|------|--------------|--------|----------------------------|
| S-MAIN | `Server.hx` | 230 | `ol-server/main.rs`, `world_boot.rs` | PARTIAL | Boot order OK; **VANILLA-ID-MAP DONE**; global mutex N/A by design |
| S-THREAD | `ThreadServer.hx` | 83 | `ol-net` accept | DONE | Tokio/async accept vs threads |
| S-TAG | `ServerTag.hx` | 84 | `ol-protocol`, `NetIntent` | PARTIAL | In-scope tags live (incl. **FLIP**); VOG/PHOTO stubs parked |
| S-HDR | `ServerHeader.hx` | 13 | — | NA | includes only |
| S-CONN | `Connection.hx` | 1079 | `ol-net`, `login_bootstrap`, `outbound`, sim send helpers, **`ai_takeover`** | PARTIAL | Login/ticket/MC/PU/MX core; **VANILLA-ID-MAP** MX/MC patch for vanilla clients (OpenLife tag keeps raw ids); **close→AI-TAKEOVER** + rlogin reclaim wired; **FERTILITY-TWINS** twin wait queue core; **LEADER-RANGE** + **CONN-PU-LEADER-FAN DONE** forced/action PU topLeader exemption; **sendSayToAllClose** **PO-MAX-DISTANCE** + **CONN-SAY-EUCLID DONE** CloseForSay Euclidean `isClose` + live `max_distance_say`; **FLIP** live `FL` |
| S-GPI | `GlobalPlayerInstance.hx` | 5343 | **`ol-sim`** (`player.rs`, `apply_intent`, `tick_vitals`) — **not** `ol-gpi` (would be a second writer / crate cycle; see ARCHITECTURE §5.1) | PARTIAL | Peel **pure** transition/combat/social formulas; **BABY** live `doBaby`; **DROP-HELD-PLAYER** live `dropPlayer` on DROP/SWAP; **BABY-NESTED-DROP** live; **GRAVE-INFO** live `GO`; **OWNER-LIST** live `OW`; **LEAD** live; **FORCE** live `receivedForce`; **LEAD-SAY** live; **FORCE-ARM** live CancleMovement wait; **FEVER-HUNGER-PE** death-path split; **FLIP** live `Connection.flip`; **FX-YUM-MULT DONE** `sendFoodUpdate` ceil prestige; types may later go to `ol-player-types` |
| **TH-ALT-OUTCOME** / alt_transition_outcome | TransitionHelper alternativeTransitionOutcome + fortification | **DONE** (core) | `alt_outcome.rs` pure + ContentDb side-tables + wall/door push(0) + OLC1 `finish_cache_boot` + live `apply_use_at` TryAgain/Proceed; residual: LiveSettings knobs; **TH-FORTIFY-APPLY DONE** |
| S-TH | `TransitionHelper.hx` | 1444 | `apply_use_at`, `apply_drop`, `multi_use`, `use_transition`, **`clothing_transitions`**, **`horse_mount`**, **`locks`**, **`alt_outcome`** | PARTIAL | Live USE=`use_transition`; held_uses; reverse/maxUse; minUseFraction; useChance; switch/force uses; loved-food polish; **TH-CLOTHING-MATRIX** core+gaps + **CLOTHING-IN-CLOTHING** + live DROP c; **TH-HORSE + HORSE-MOUNT-POLISH + HORSE-EAT-ILL**; **TH-LOCK + LOCKPICK-SETTINGS**; **IS-CLOSE** + **GPI-TOO-CLOSE**; **CLOTHING-CONTAIN-SIZE** + **USE containerIndex / L1087**; **TH-ALT-OUTCOME DONE (core)**; **NEVER-DROP-CMD** + **WOUND-CMD** + **HOLDING-PLAYER-CMD** + **CLEAR-WRITING** + **READ-WRITING** + **BASKET-PILE-CMD** + **AI-BLOCK-CMD** + **TH-ALT-LIVE-KNOBS** + **MIN-PICKUP-AGE DONE**. Gaps: client OLC1 v8 bake (parked). **SWAP** live ground swap DONE |
| **JUMP-BW-FULL** | `MoveHelper` + `GPI.jump` + Server JUMP | — | `jump_bw` + `apply_player_jump` + MOVE gates | **DONE** | pure plan + live JUMP/AI/MOVE; **JUMP-DROP-TRANSFORM DONE** (payload xy ignored; held `dropPlayer` at carrier tile) |
| S-MOVE | `MoveHelper.hx` | 705 | `ol-move-rules` (`speed`/`close_exact`/`action_range`/`math_wrap`) + `ol-sim` `move_path`/`move_live_gates`/`move_notes`/`apply_move_*` | **DONE** (+ **OL-MOVE-RULES** + **OL-MOVE-RULES-SPEED** peels) | pure speed/range/wrap in `ol-move-rules`; sim re-exports + bloody held override; `player_move_speed` / path-start use `apply_calculate_speed_full*`; **HALF-PENALTY-STRONG DONE** Noble+ contained half-penalty; **BACKPACK-NEST-DUAL DONE** clothing[5] nest↔flat; **CONN-PU-LEADER-FAN DONE** forced/action PU leader exemption |
| S-TIME | `TimeHelper.hx` | 2204 | tick_* + environment + `world_time` + `animal_move` + `animal_damage` + **`animal_pop`** + `long_term` + settings hot-reload + `contained_timers_persist` + **`nested_timers`** + **`fever_pe`** + **`score_age`** | PARTIAL | **TIME-WORLD** + **TIME-LONG** + **damage_escape** + **chase_biome** + **pop_die_offspring** + settings reload + **CONTAINED-TIMERS-PERSIST** + **NESTED-IN-NESTED-TIMERS** + **FEVER-EMOTE** + **CURSED-GRAVES-INDEX** + **CURSED-GRAVE-TELEPORT** (`!TCG`/`!TV` consumers); **ANGRY-TIME-MIN DONE**; **SEARCH-HOME-OVEN DONE** (local r=80 + torus quad); **COMBAT-MOSQUITO-KIND DONE** (live wire); **SCORE-AGE-58 DONE** (trueAge 58 GM) |
| FEVER-EMOTE | `TimeHelper.UpdateEmotes` + `GPI.hasYellowFever` / `isSuperHot` / `doEmote` | — | `ol-sim/fever_pe.rs` + `apply_update_emotes_tick` + feed ill PE | **DONE** | pure ladder yellowFever(7)/heatStroke(21); live tick%30; ambient 9s; feed-ill PE; **FEVER-HUNGER-PE DONE** (living PE 1 at food&lt;3; Haxe PE 31 needs food&lt;0, Rust death at food&lt;0) |
| S-WMAP | `WorldMap.hx` | 1288 | `ol-world` persist + postload + **`ol-sim/world_food_stats`** + **`object_counts_share`** + ObjectCounts pure | PARTIAL → postload + **WORLD-FOOD-FACTOR** + **FOODSTATS-DISK** + **EATEN-FOOD-PCT** + **OBJECTCOUNTS-LIVE** + **LINEAGE-24H** | OLW3 + live eaten% + FoodStats dump + **ObjectCounts census DONE**; **getStarvingFoodFactor** 24h DONE |
| S-TEMP | `TemperatureHandler.hx` | 201 | `environment`, `heat_ideal`, `world_time`, **`map_temp_player`** | **DONE** (player wire) | Map-slice + insulation; **BalanceTemperatureArea live**; closest-heat + clothing rValue + color/biome-love/storedWater/held-by + warm/cold places + **HX foodDrainTime** |
| S-BIOME | `Biome.hx` | 157 | `ol-world/biome`, `biomes_query` | PARTIAL | IDs; deep river / special biomes TODOs in Haxe too |
| S-LIN | `Lineage.hx` | 530 | `lineage_persist`, relations, **`prestige`**, **world_food_stats LINEAGE-24H** | PARTIAL → **CLASS-BONI + LINEAGE-24H + LINEAGE-ARCHIVE + LINEAGE-BIRTH-TIME + LINEAGE-LAST-SAID + LINEAGE-EVE-ID + LINEAGE-REP-DISK + LINEAGE-COINS-DISK + LINEAGE-FAMILY-NAME + LINEAGE-PO-ID + LINEAGE-ACCOUNT-ID + LINEAGE-DYNASTY + LINEAGE-FOLLOW-ID + LINEAGE-KILLED-BY + LINEAGE-TRUE-AGE DONE** | OLN14 trueAge + OLN13 killedByPlayerId + OLN12 followPlayerId + OLN11 myDynastyId + OLN10 accountId + OLN9 po_id + OLN8 familyName + OLN7 coins + OLN6 reputation + OLN5 myEveId + OLN4 lastSaid + OLN3 birthTime + deathTime/reason; PrestigeClass; **reasonKilledLastDay 24h** + boot seed stamps; **LINEAGE-ARCHIVE** skip-on-save + `{path}.archive`; skip unused **houseGeneration** (Haxe not set yet) |
| S-ACC | `PlayerAccount.hx` | 280 | `accounts`, `account_persist`, `postload_wire` | PARTIAL | **OLA2** numeric `PlayerAccount.id` (`AccountIdIndex`) + **GRAVE-ACCOUNT-ID** graves stamp `rec.id` (FNV legacy OLW) + OLA1 lives/score + session graves + family_prestige + score_entries + **WEB-ACCOUNTS-STATS** female/male/is_ai snapshot |
| S-SOUL | `PlayerSoul.hx` | 477 | `player_soul` + `player_soul_wire` + `soul_live` + `Player.soul` | **PARTIAL → AI-SOUL-WIRE DONE** | Pure FIFO + prompts; sticky soul; **PLAYER-MALE DONE**; **SOUL-LIVE-CAPS DONE** (`AiMemoryMaxEntries` 20 / `AiChatMemoryMaxEntries` 100 Live); **SOUL-CHAT-ENTRY skip** Haxe `toSoul.addChatEntry` is Haxe TODO (no live call); `fromSoul` live on LLM fan |
| S-NAME | `NamingHelper.hx` | 337 | `naming.rs` | **DONE** (core) | **AI-LASTNAMES** 2-letter `NameIndex` + 20-try rng mutate; curated + cwd `lastNames.txt` OnceLock (tests skip file). **YOU ARE** gender-split live (`plan_do_naming_you_are_ex_gender`). Residual: full OHOL txt not in repo |
| S-SCORE | `ScoreEntry.hx` | 98 | `score.rs` + **`score_entry.rs`** | **DONE** | SES1 prestige queue; **SCORE-MALI DONE** (`CursedGraveMali` Live + overflow wire) |
| S-WEB | `WebServer.hx` | 350 | `ol-web` | PARTIAL | Viewer/APIs; **FOODSTATS-WEB** `/stats/food`; **WEB-ACCOUNTS-STATS** `/stats/accounts` DONE; lineage death-reason HTML (**WEB-LINEAGE-STATS**) |
| S-SER | `SerializeHelper.hx` | — | `ol-world` / nested persist | **DONE** | **NESTED-PERSIST-SER**: OLW3 NestedHelper recursive contained + uses + ground_id (`olw3_slot_meta_and_owners_roundtrip` / `nested_helper_write_read_pure`). SerializeHelper.hx is GPI RTTI codegen only. |
| S-AI | `ServerAi.hx` | — | selfplay / npc_ai | PARTIAL | Thin vs AiBase; **PATH-REACH-MERGE** npc pull-once/push + AI-TAKEOVER push |
| S-AIH | `AiHandler.hx` | — | `ai_handler` pure | **DONE** (pure) | LLM prompt pure; live wire stack separate |
| S-AIP | `AIProvider.hx` | — | `ai_provider` HTTP | **DONE** (HTTP) | MiniMax/Anthropic drain |

---

## B. `openlife/auto/`

| ID | Haxe file | Rust targets | Status | Notes |
|----|-----------|--------------|--------|-------|
| A-BASE | `AiBase.hx` | **`ol-ai-helper`** + **`ol-ai-professions`**; live **`ol-sim`** `profession_scan` / **`ol-server`** `npc_ai` | PARTIAL | **Line pointer:** [AIBASE_MIGRATION.md](AIBASE_MIGRATION.md) (chunk walk from L0). Pure SMs in professions crate; live USE/DROP I/O still sim. **NPC-IGNORED-FLOOR** + **AI-JOB-GRAVE** + **AI-HANDLE-DEATH** + **AI-REMOVE-CONTAINER** DONE |
| A-HELP | `AiHelper.hx` | **`ol-player-helper`** + **`ol-ai-pathing`** + **`ol-ai-crafting`**; live scan in sim | PARTIAL | **SEARCH-BEST-FOOD** + PATH-REACH + PathfinderNew live Goto |
| A-AI | `Ai.hx` / `AiPx.hx` | thin | PARTIAL | |
| A-PF | `Pathfinder*.hx` | **`ol-ai-pathing`** `pathfinder_new.rs` | **DONE** (core) | PathfinderNew + 100ms **AI-PATHFINDER-TIMEOUT**; unused full-grid brute / `WriteMapToFile` omitted |
| A-ACT | `Action.hx` + `actions/*` | intent / action enums | PARTIAL | |
| A-REST | other auto modules | various | PARTIAL / NA | See TODO_PORT AI section |

---

## C. `openlife/settings/`

| ID | Haxe file | Rust targets | Status | Notes |
|----|-----------|--------------|--------|-------|
| C-SS | `ServerSettings.hx` | **`ol-config`** knobs + **`ol-content` `patches/`** for PatchObjectData/PatchTransitions | PARTIAL | Knobs: LiveSettings. **Patches are not settings** — `apply_all_haxe_content_patches`. Residual knobs: plant-offspring / spawnAsChild pairing (`ColdSeasonTemperatureFactor` **live**). |
| C-SET | `Settings.hx` / `OpenLifeData.hx` | **`ol-config`** | PARTIAL | |

## C2. `openlife/data/` (content, not server tick)

| ID | Haxe file | Compile unit | Status |
|----|-----------|--------------|--------|
| D-OBJ | `object/ObjectData.hx` | **`ol-content`** | DONE core |
| D-HELP | `object/ObjectHelper.hx` | **`ol-world`** NestedHelper + **`ol-sim`** `nested_body` | PARTIAL |
| D-TR | `transition/TransitionData.hx` / `TransitionImporter.hx` | **`ol-content`** + **`ol-binary`** OLT1 | DONE core |
| D-CAT | `transition/Category.hx` | **`ol-content`** | DONE core |
| D-PI | `object/player/PlayerInstance.hx` | **`ol-protocol`** PU + **`ol-sim`** `Player` | PARTIAL |
| D-MAP | `map/MapData.hx` | **`ol-world`** | PARTIAL |

---

## D. High-value chunk ids (subset; see TODO_PORT for full)

| Chunk ID | About | Status | Notes |
|----------|-------|--------|-------|
| **FOODSTATS-DISK** / foodstats_txt | WorldMap.writeFoodStatistics FoodStats dump | **DONE** | `WorldFoodShare` + autosave/shutdown; HTML → **FOODSTATS-WEB** |
| **FOODSTATS-WEB** / food_stats_html | WebServer.generateFoodStatistics `/stats/food` | **DONE** | `WebState.food_view` = live WorldFoodShare; pure HTML Food/Eaten/Related |
| **OBJECTCOUNTS-LIVE** / object_counts_share | WorldMap TraceCountObjectsToDisk ObjectCounts dump | **DONE** | share + `count_objects_from_world` / `update_object_counts` nest census; boot seed; periodic recompute; autosave `ObjectCounts.txt` |
| **EATEN-FOOD-PCT** / world_food_map | live eaten% on add | **DONE** | |
| **WORLD-FOOD-FACTOR** | getFoodFactor / starving | **DONE** (core) | lineage 24h → **LINEAGE-24H DONE** |
| **LINEAGE-24H** / starving_window | reasonKilledLastDay 24h + getStarvingFoodFactor | **DONE** (core) | OLN2 death fields + boot seed; full stats + HTML + `/stats/lineage` wire; **LINEAGE-ARCHIVE DONE**; **LINEAGE-BIRTH-TIME DONE** living ages from `birthTime` |
| GPI-FOOD / CRAVING / SEARCH-BEST-FOOD / C-SS-FULL-TABLE | yum/meh + craving + search + FoodFactor Live | DONE | foodObjects id-sorted + skip dummies |
| GPI-DEATH / place_grave / PLACE-OBJECT | death/grave/place | **DONE** (core) | **GRAVE-ACCOUNT-ID** numeric `PlayerAccount.id` on `owners_by_account`; FNV only for pre-OLA2 OLW |
| **WALLET-COINS** / take_coins | takeCoins wallet on wound/damage | **DONE** | coins_stolen + economy gift; HIT lethal+equip; **WALLET-I32-FLOOR DONE**; **HEALTH-PRESTIGE-FAN DONE**; **WALLET-PERSIST-RESTORE DONE**; **PRESTIGE-CLOTH-FACTOR DONE** clothing prestigeFactor + crown extra |
| **DARK-NOSAJ** / dark_nosaj_use | Tarr 3112 + Dark Nosaj 2466 USE set/clear | **DONE** | pure `plan_monument_use` + damage×1.2 + eat prestige gate; Player.praised_jinbali; single USE side-effects + CU/say; **WALLET-I32-FLOOR DONE**; **FX-YUM-MULT DONE** eat/feed FX ceil prestige; monument-only FX still optional |
| PLAYERS-BIN / NESTED-* / SOCIAL-WAR-PERSIST | persist shares | **DONE** (core) | **WALLET-PERSIST-RESTORE DONE**; **YUM-MULT-PERSIST DONE** PLB yum_multiplier → lineage/combat prestige |
| LEADER-RANGE / FOLLOW-HIRE / MAP-LOCATION-PINS / PO-FAR | social UX | **DONE** (core) | |
| **PO-MAX-DISTANCE** / close_say_range | MaxDistanceToBeConsideredAsCloseForSay=20 | **DONE** | `ADULT_CHAT_RANGE`≠`NEARBY_RANGE`; pending-follower spoken_says + social-pin/coins/moskitos say fans; ModuleConst residual |
| **CURSED-GRAVES-INDEX** / cursed_graves | ClearCursedGraves + ovens + GetClosestBoneGrave + CursedGraveTime | **DONE** (core) | `WorldMapTimeState.cursed_graves`/`ovens`; map-slice fill + step%2000 prune; wolf global closest; sharp-stone extra LiveSettings `CursedGraveTime`; residual SearchNewHome local oven + CreateScoreEntryForCursedGrave on overflow pop |
| **CURSED-GRAVE-TELEPORT** / tcg_tv_teleport | GPI `!TCG`/`!CURSEDGRAVE` + `!TV`/`!VILLAGE` + teleport/doTeleport | **DONE** | pure `teleport_cmd` + **TCG-LIVE-WIRE** SAY `try_apply_teleport_bang` + torus pick + JumpToNonBlocked + MC/PU (VOG parked); godmode / `can_use_server_commands`; HashMap keys stay `x+y*width` (Haxe array `y-1` not used) |
| COMBAT-BLOODY / WEAPON-* / ALLY / PRESTIGE-ALLY / REPUTATION-HIT / FEVER-BLEED / **WALLET-COINS** | combat | **DONE** (core) | **HIT-PRESTIGE-COST DONE** yum debit + GM on connecting HIT |
| **COMBAT-MOSQUITO-KIND** / mosquito_animal | Mosquito Swarm 2156 AnimalKind + biomeLove(JUNGLE) | **DONE** | `AnimalKind::Mosquito` path damage=1; pure jungle love; path moskito scale + spawn×2 + chase `is_deadly_animal` |
| **MOSQUITO-MAPCHANCE** / mosquito_mapchance | BiomeAnimalHitChance live + 2156 mapChance/SWAMP | **DONE** | `apply_animal_path_damages` miss gate; `is_animal_*_deadly_for_me`; content `mapChance*=0.3` + SWAMP + biome_spawn rebuild |
| TIME-ANIMAL-* / TIME-LONG / MAP-TEMP-PLAYER | world tick | **DONE** (core) | |
| **MOVE-VOG-WRAP** / cancel_wrap | VOG on CancleMovement + world-wrap fold | **DONE** | `fold_relative_around_world` / `fold_world_pos_around_world` / `cancel_movement(use_vog)` + always MC; wrap gate on path start; jump/blocked `quad>25` VOG |
| **C-SS-AGE-FOOD** / age_food_max | NewBorn/OldAge FoodStoreMax Live | **DONE** | LiveSettings + GameplayKnobs + `FoodStoreMaxKnobs` pure bands; spawn/vitals/eat; **C-SS-AGE-FOOD-COMBAT** HIT + **SUPERMEH-FOOD-MAX** + **ANIMAL-DAMAGE-FOOD-PIPE** |
| AI-PRIO / AI-JOB-* / CRAFT-LIVE / AI-LLM-* | AI | PARTIAL | **NPC-IGNORED-FLOOR** + **CRAFT-LIVE-IO** + **DROP-HELD-QUIVER** + **SHORTCRAFT-MAX-NEW-ACTOR** + **AI-JOB-SMITH-RESID** + **SMITH-CHISEL-PLAYER-CACHE** + **SMITH-LADDER-PEER-KIND** + **CLOTHING-HAS-TAILOR** + **FIRE-PLACE-STICKY** + **FIRE-BEST-AI** + **FIRE-CRAFT-R30** + **AI-JOB-GRAVE** + **AI-HANDLE-DEATH** + **AI-REMOVE-CONTAINER** + **AI-HANDLE-TEMP** + **AI-JOB-HUNT** + **AI-JOB-LUMBER** + **AI-JOB-COLLECT** + **AI-JOB-WATER** + **AI-JOB-FOODSERVER** + **AI-JOB-TAILOR** + **AI-FEED-MID** + **FILL-BUCKET** + **YOU-ARE-PROF** + **DO-WATERING-LOW** + **DO-CARROT-LOW** + **FILL-BEAN-BOWL** + **FILL-BEAN-HELD** + **FILL-BERRY-HELD** + **PULL-CARROT-ROW** + **SPAWN-QUEUE-POLICY** + **VANILLA-ID-MAP** + **HALF-PENALTY-STRONG** + **HIT-STOP-MOVE** + **HIT-BLOCK-NONALLY-MOVE** + **BABY-BONES-ARMS** + **RECENT-EXILE-ALLY** + **NAME-FULL-LINEAGE** + **LOCATION-SAYS-MARKERS** + **AGE-10-FATHER-LIVE** + **SPRINGS-TAR-RESPAWN** + **WELLS-OIL-DECAY** + **EVE-DEADLY-ANIMALS** + **OWNED-GATE-DELETE** DONE. Next **SCORE-AGE-58**; see TODO_PORT |
| **AI-JOB-GRAVE** / is_handling_graves | `AiBase.isHandlingGraves` GRAVEKEEPER | **DONE** | pure SM + live scan/ladder/npc; getBestAi vs grave; shovel then hoe. Residual own-grave owner fill |
| **AI-HANDLE-DEATH** / handle_death | `AiBase.handleDeath` | **DONE** | MaxAge−2 GRAVEKEEPER wipe + graves/home/drop; player+npc. Residual isRemovingFromContainer |
| **AI-REMOVE-CONTAINER** / is_removing_from_container | `AiBase.isRemovingFromContainer` | **DONE** | sticky expected parent; drop/goto/REMV; graves cargo stages |
| **AI-HANDLE-TEMP** / handle_temperature | `AiBase.handleTemperature` | **DONE** | drink/GetOrCraft/biome/fire/arrive; nested isHandlingFire(2); npc 1b |
| **AI-JOB-HUNT** / is_hunting | `AiBase.isHunting` HUNTER | **DONE** | assigned 100 + mid age>14; knife+snake then firebrand+mosquito; home quad < 400 |
| **AI-JOB-LUMBER** / is_cutting_wood | `AiBase.isCuttingWood` LUMBERJACK | **DONE** | assigned 100 + low-priority; firewood 344 then butt log 345 GetCraftAndDrop; firePlace required |
| **AI-JOB-COLLECT** / is_collecting | `AiBase.isCollecting` COLLECTOR | **DONE** | assigned 100 + low-priority isCollecting(1); kindling/bushes/rabbits/mutton; scan r=60 |
| **AI-JOB-WATER** / do_watering | `AiBase.doWatering` WATERBRINGER | **DONE** | assigned 100 closest dry r=30; mid doWatering(3) list-order kept; low doWatering(1) **DO-WATERING-LOW** |
| **FILL-BUCKET** / fill_bucket_if_needed | `AiBase.fillBucketIfNeeded` | **DONE** | mid WATERBRINGER max=1 drop/tank/source |
| **DO-WATERING-LOW** / do_watering_1 | `AiBase.doWatering(1)` | **DONE** | max=1 closest dry r=30; LowPriorityWork + AgeRotatedJob; residual makeFood doWatering(1) |
| **DO-CARROT-LOW** / do_carrot_farming_1 | `AiBase.doCarrotFarming(1)` | **DONE** | max=1 after watering-low; assigned 100 separate; residual doCriticalStuff doCarrotFarming(1) |
| **FILL-BEAN-BOWL** / fill_bean_bowl_if_needed | `AiBase.fillBeanBowlIfNeeded()` | **DONE** | low green beans after carrot-low; BowlFiller pickup/GetItem; residual mid onlyFillHeld |
| **FILL-BEAN-HELD** / fill_bean_bowl_held | `AiBase.fillBeanBowlIfNeeded(*, true)` | **DONE** | mid onlyFillHeld green then dry before isHandlingFire |
| **FILL-BERRY-HELD** / fill_berry_bowl_held | `AiBase.fillBerryBowlIfNeeded(true)` | **DONE** | mid held 253 on closest bush r=20 before bean held |
| **PULL-CARROT-ROW** / pull_carrot_row | `AiBase.shortCraft(0, 400, 10)` | **DONE** | mid empty-hand USE carrot row r=10; no profession; seed guard |
| **DIE-SCORE-SKIP** / try_allow_voluntary_die | `Connection.die` L840 | **DONE** | /DIE prestige gate; score not debited; residual food_store -= 100 |
| **SPAWN-QUEUE-POLICY** / spawn_queue | `Connection.loginHelper` L135-138 | **DONE** | MaxPlayers cap; cull AIs; score/last-life; IP new-account + spam |
| **YOU-ARE-PROF** / you_are_profession | hearer `YOU ARE SMITH` assign | **DONE** | job runtimes + follower gate; not DoNaming first-name |
| **AI-JOB-FOODSERVER** / is_feeding_player_in_need | `AiBase.isFeedingPlayerInNeed` FOODSERVER | **DONE** | assigned 100 GetCloseStarvingPlayer + feed/SearchBestFood |
| **AI-FEED-MID** / feed_player_in_need_max1 | mid `isFeedingPlayerInNeed()` max=1 | **DONE** | SMITH<1 sensor + FeedPlayerInNeed rung; residual YOU ARE / npc cands |
| **AI-JOB-TAILOR** / craft_clothing | `AiBase` assigned TAILOR clothing | **DONE** | high + medium/low(100) ladder/scan/npc |
| **AI-JOB-SMITH-RESID** / smith_chisel_resid | `AiBase` smith + `PatchObjectData` Chisel | **DONE** | GetCraftAndDrop fill + player-path Chisel cache + per-kind ladder peer_count |

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

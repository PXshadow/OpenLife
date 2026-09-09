# Server port workflow queue

**Next:** **PAUSED**. Do not pick until the user unpauses. Skip Haxe-only TODOs ([`HAXE_TODOS_OUT_OF_SCOPE.md`](HAXE_TODOS_OUT_OF_SCOPE.md)).

**Changelog (2026-09-09):** **AI-ATTACK-PLAYER** live. Picker **PAUSED**. **SEASON-REGROW-DECAY** / **AI-KNIFE-STUFF** live.

## Concurrency policy (active)

**Worker + 60s watchdog.** One leftover per fire. Max **1 heavy `ol-sim` Act**. Fresh `spawn_subagent` (never `resume_from`). Watchdog resumes only if stuck and SuperGrok used % is **below** `resume_below_percent` in `%USERPROFILE%\.grok\openlife-port-watchdog\watchdog.toml` (default **50**). No full-repo rescan.

| Role | ID | Interval | Job |
|------|----|----------|-----|
| Watchdog | **off** | — | — |
| Worker | none | — | **PAUSED** |

**Stuck** = In flight **WORKER** is not `RUNNING`, or heartbeat older than **45 minutes**, and Next is not IDLE-COMPLETE.

**Out of scope:** PHOTO, VOG, multi-server twins, SQL, mutex/debug knobs, unused ModuleConst, **Haxe TODOs that never shipped** (see [`HAXE_TODOS_OUT_OF_SCOPE.md`](HAXE_TODOS_OUT_OF_SCOPE.md)).

---

## In flight

**WORKER:** none  
**State:** PAUSED  
**Heartbeat:** 2026-09-09T20:40:00Z

Token-free watchdog: `%USERPROFILE%\.grok\openlife-port-watchdog\watchdog-loop.ps1`. Grok scheduler is **off**.

---

## Resume queue — non-AI (working Haxe only)

Skip if grep shows live. Do **not** pick Haxe-only TODOs.

| # | matrix_id | About | Haxe |
|---|-----------|--------|------|
| 1 | **DISPLAY-STUFF** | Human tick%50: map chunk / followings / close players / yum food / deadly animals | `TimeHelper.DisplayStuff` ~262 |
| 2 | **DO-LEADERSHIP-TICK** | Human tick%40 `DoLeadership` exile/trust pulse | `TimeHelper` ~253 |
| 3 | **SPAWN-INDEX-SCAN** | Map-slice berry/banana/carrot/cactus/garlic/oven/grave/road indexes | `DoWorldMapTimeStuff` ~1098 |

---

## Resume queue — AI (working Haxe functions)

Verify with grep; skip if already live. Assigned jobs (TAILOR/HUNT/LUMBER/COLLECT/WATER/FOODSERVER/GRAVE/FARM/SMITH/BAKER/POTTER/FIREKEEPER/FIREFOOD) and **AI-KNIFE-STUFF** / **AI-ATTACK-PLAYER** are **DONE**. Do not re-pick those cores. Skip Haxe TODOs inside these bodies (port-as-is).

| # | matrix_id | About | Haxe |
|---|-----------|--------|------|
| 1 | **AI-ATTACK-PLAYER** | Combat: getWeapon + range walk + `kill` | `attackPlayer` ~5822 |
| 2 | **AI-KILL-ANIMAL** | Live killAnimal | `killAnimal` ~5878 |
| 4 | **AI-CRITICAL-STUFF** | After assigned jobs: floor under home/kiln/forge, bushes, cleanUp, watering/farm/firefood/bake/pottery/carrot | `doCriticalStuff` ~6072 |
| 5 | **PLACE-FLOOR-UNDER** | Floor under home / kiln / forge | `placeFloorUnder` ~1058 |
| 6 | **KEEP-BUSHES-ALIVE-LIVE** | Soil on dying bush when <20 domestic bushes | `keepBushesAlive` ~6052 |
| 7 | **CLEANUP-LIVE** | Wire `cleanup_profession` / `pile_up` on lumber + critical + pottery | `cleanUp` / `pileUp` ~943 |
| 8 | **CUT-WOOD-UNASSIGNED** | `isCuttingWood()` max=1 after smith (not assigned LUMBERJACK) | ~808 |
| 9 | **SMITH-UNASSIGNED** | lastProfession SMITH + late `doSmithing()` | ~770 / ~811 |
| 10 | **COLLECT-UNASSIGNED** | `isCollecting(1)` before watering | ~760 |
| 11 | **JOB-BY-AGE** | Age/5 rotation: berry, basic farm, bake, pottery, sheep | ~793–802 |
| 12 | **SHEEP-WIDE** | `isSheepHerding()` after rotation, wider radius | ~807 |
| 13 | **CLOTHING-LOW-AGE30** | `craftLowPriorityClothing` when age>30 | ~817 |
| 14 | **ADV-FARM-LOW** | `doAdvancedFarming(1)` then `(2)` | ~818 / ~834 |
| 15 | **CRAVING-CRAFT** | `getCraving` craft if home count<2 (skip 31/1121/1471) | ~820–827 |
| 16 | **FIREFOOD-UNASSIGNED** | `makeFireFood(1)` on makeStuff path | ~833 |
| 17 | **MAKE-STUFF** | Residual popcorn BowlFiller + Defer* tails | `makeStuff` ~4074 |
| 18 | **MOVE-HOME-IDLE** | `isMovingToHome(4)` then drop held then idle say | ~840–873 |
| 19 | **GET-CRAFT-AND-DROP** | GetCraftAndDropItemsCloseToObj (kindling/firewood/collect) | ~893 |
| 20 | **WATER-SOURCE-IDS** | Dynamic WaterSourceIds vs static bucket sources | craft specials |
| 21 | **AI-CRAFT-MULTI-SPECIALS** | craftItemHelper adze/bucket specials + retarget filters | `craftItemHelper` |
| 22 | **MAKE-FIREWOOD** | `makeFireWood` firekeeper cascade residual | ~4427 |
| 23 | **MAKE-POPCORN** | `makePopcornIfNeeded` BowlFiller peer | ~4281 |
| 24 | **HANDLE-MILK-NEST** | Nested milk (shepherd/baker handle_milk live residual) | `handleMilk` ~1774 |
| 25 | **COMPOST** | `doComposting` soil-maker tail | ~2056 |
| 26 | **QUIVER-LIVE** | Wire `plan_fill_up_quiver` on tailor/clothing mid | `fillUpQuiver` ~4632 |
| 27 | **USE-UP-DOUGH-LIVE** | Wire `use_up_dough` before fire/oven drop | `UseUpDough` ~5237 |
| 28 | **HIRE-COINS-LIVE** | Hire debit coins (Haxe subtracts cost) | hire AI |
| 29 | **AUTO-FOLLOW-PLAYER** | Closest-human acquire when AutoFollowPlayer (Haxe default false) | `isMovingToPlayer` ~8284 |
| 30 | **ESCAPE-LIVE-IO** | Multi-try flee path I/O | `escape` ~6493 |
| 31 | **STAY-CLOSE-CHILD** | Live `isStayingCloseToChild` | ~6399 |
| 32 | **FEED-CHILD-LIVE** | Live `isFeedingChild` | ~6412 |
| 33 | **CONSIDERING-MAKE-FOOD** | `isConsideringMakingFood` | ~8466 |
| 34 | **POTTERY-CLEANUP-AGE** | Pottery `cleanUp` age%3 gate | `doPottery` |
| 35 | **PATHFINDER-NEW-LIVE** | Use PathfinderNew (100ms budget) on NPC/player path | `PathfinderNew.hx` |
| 36 | **DROP-HELD-QUIVER** | Smart drop quiver default-empty on npc | drop held |
| 37 | **CRAFT-COUNTDONE-REQUEUE** | Interrupted countDone re-queue | `craftItemHelper` ~6677 |
| 38 | **MOTHER-LINEAGE-SENSORS** | Mother lineage / ordered follow sensors | `doTimeStuffHelper` |

---

## Parked (do not pick)

PHOTO · VOG · multi-server twins · SQL · mutex/debug/secret knobs · unused ModuleConst / SETTINGS-LONG-TAIL · ticket (toml toggle) · Haxe `TODO`s that never shipped ([`HAXE_TODOS_OUT_OF_SCOPE.md`](HAXE_TODOS_OUT_OF_SCOPE.md)) · `OL-AI-SPLIT-P3` (Rust arch).

---

## Done recently

**AI-ATTACK-PLAYER** 2026-09-09. **SEASON-REGROW-DECAY** 2026-09-09. **AI-KNIFE-STUFF** 2026-09-09. Haxe-only TODOs are **not** on the picker. Assigned AI jobs already live: TAILOR, HUNT, LUMBER, COLLECT, WATER, FOODSERVER, GRAVE, FARM, SMITH, BAKER, POTTER, FIREKEEPER, FIREFOOD.

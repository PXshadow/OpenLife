# Server port — highest first

**Date:** 2026-09-09  
**Workspace:** `openlife/RustServer`  
**Authority:** [`QUEUE.md`](QUEUE.md) **Resume queue** is what the leftover loop picks; this file summarizes why. [`TODO_PORT.md`](TODO_PORT.md) for the full checklist; [`ARCHITECTURE.md`](../ARCHITECTURE.md) for crate rules.

Crate split / wirer freeze is **done**. Playable core is **largely done**. Remaining work is Haxe leftovers, not more writer crates.

**Do not:** PHOTO, VOG, multi-server twins, SQL, mutex/debug settings, unused ModuleConst, further AiBase rungs until QUEUE Resume queue is empty.

---

## Now (scheduler pick list)

**only** these non-AI leftovers (same table as [`QUEUE.md`](QUEUE.md)). Skip if grep shows live.

| # | Id | What |
|---|-----|------|

**Resume table empty.** Next **IDLE-COMPLETE**. Do **not** restart AiBase rungs unless QUEUE says so.

Parked (do not pick): PHOTO, VOG, multi-server twins, SQL, SETTINGS-LONG-TAIL / ModuleConst.

---

## Status of #1–#10

**DONE (2026-09-01):**
1. ColdSeasonTemperatureFactor Live  
2. CRAFT-LIVE-IO `useHeldObjOnTarget` staging  
3. P0-IS-CLOSE squared `isClose` on USE/DROP/REMV/HIT/FEED/PUSH/HOLD/craft staging + protocol `KILL` killHelper + HIT `deadlyDistance²+0.1`  
4. TIME-LONG live tick + nested multi-use / contained decay  
5. CRAFT-SPECIALS (countDone + specials already live)  
6. SPAWN-AS-CHILD Eve pairing wire (`last_ai_eve` / `last_human_eve`)  
7. PLANT-OFFSPRING from living plants (compiled 0.05)  
8. WEB-LINEAGE product pages (list + character + stats)  
9. Parity tests: treat remaining `ol-sim` fails as Haxe sensors  
10. Needle/thread: document-only  

**Optional polish DONE (2026-09-01):** LiveSettings `GrowNewPlantsFromExistingFactor` / `MaxPlayersBeforeStartingAsChild`; protocol `KILL x y [id]` killHelper (angryTime, deadly float, click-tile, unarmed-ally warn/exile, exactTx, DoDamage); HIT max = `deadlyDistance² + 0.1` (name table only when deadly ≤ 0.1).

**Playable leftovers DONE (2026-09-01):** foodObjects id-order; TH-MULTI clothing tool last-use; lineage HTML eve/lastSaid/death; needle/thread **port-as-is**. AI job cores already live.

**In-scope live-path follow-ups DONE (2026-09-01):** YOU ARE gender-split names; live `UpdateEmotes` (`tick % 30`); npc `fill_live_sensors` + escape; path `isAnimalDeadlyForMe` (loved biome / weapon); popcorn BowlFiller peer.

**2026-09-02:** NPC Goto `isAnimalDeadlyForMe`; hiddenWound + lostCombatPrestige; closest-heat + held heat ambient; per-kind profession peer_count; foodTarget/lastGoto Player↔NPC sync; **clothing rValue matrix**; **color/biome-love/storedWater/held-by**; **warm/cold place fitness** + NPC superbad goto; **HX foodDrainTime**; **TH-CHEST-COINS**.

Parked: PHOTO, VOG, multi-server twins, SQL. Accepted remainder: ~300 Haxe ModuleConst knobs (not Live). **SELF-EAT** + **UBABY** + **SWAP** + **BABY** + **DROP-HELD-PLAYER** + **BABY-NESTED-DROP** + **GRAVE-INFO** + **OWNER-LIST** + **LEAD** + **FORCE** + **LEAD-SAY** + **FORCE-ARM** + **FEVER-HUNGER-PE** + **FLIP** + **ANGRY-TIME-MIN** + **PLAYER-MALE** + **SEARCH-HOME-OVEN** + **SCORE-MALI** + **CONN-SAY-EUCLID** + **BACKPACK-NEST-DUAL** + **CONN-PU-LEADER-FAN** + **JUMP-DROP-TRANSFORM** + **TCG-LIVE-WIRE** + **SOUL-LIVE-CAPS** + **WALLET-I32-FLOOR** + **LINEAGE-ARCHIVE** + **HEALTH-PRESTIGE-FAN** + **WALLET-PERSIST-RESTORE** + **PRESTIGE-CLOTH-FACTOR** + **HIT-PRESTIGE-COST** + **YUM-MULT-PERSIST** + **FX-YUM-MULT** + **LINEAGE-BIRTH-TIME** + **LINEAGE-LAST-SAID** + **LINEAGE-EVE-ID** + **LINEAGE-REP-DISK** + **LINEAGE-COINS-DISK** + **LINEAGE-FAMILY-NAME** + **LINEAGE-PO-ID** + **LINEAGE-ACCOUNT-ID** + **LINEAGE-DYNASTY** + **LINEAGE-FOLLOW-ID** + **LINEAGE-KILLED-BY** + **LINEAGE-TRUE-AGE** + **GRAVE-ACCOUNT-ID** + **C-SS-AGE-FOOD-COMBAT** + **SUPERMEH-FOOD-MAX** + **ANIMAL-DAMAGE-FOOD-PIPE** + **PLACE-OBJECT-SPILL** + **TIME-OVERFLOW-PLACE** + **TIME-DECAY-PLACE** + **FEED-OTHER-EAT** + **FEED-OTHER-EMOTE** + **EAT-RESPONSIBLE-PU** + **EAT-REFUSE-EMOTE** + **HORSE-EAT-EMOTE** + **HORSE-EAT-FX** + **HORSE-EAT-PRESTIGE** + **EAT-ILL-EMOTE** + **HORSE-EAT-ILL** + **HORSE-EAT-REFUSE-SWAP** + **HORSE-EAT-GAIN-NONE DONE.** **SOUL-CHAT-ENTRY** skip (Haxe `toSoul.addChatEntry` TODO; `fromSoul` live). **ALLY-PICKUP-THRESHOLD** + **GRAVE-TOUCH-PLAYERS** + **GRAVE-TOUCH-DROP** + **GRAVE-TOUCH-REMV** + **ALLY-PICKUP-DROP** + **KILLMODE-DROP** + **NEVER-DROP-CMD** + **WOUND-CMD** + **HOLDING-PLAYER-CMD** + **CLEAR-WRITING** + **READ-WRITING** + **BASKET-PILE-CMD** + **AI-BLOCK-CMD** + **TH-ALT-LIVE-KNOBS DONE.** Named leftover: **SETTINGS-LONG-TAIL** (skip mutex/debug/secret/Haxe-no-op; ~300 ModuleConst not Live — do **not** implement). **AI-JOB-GRAVE** + **FIRE-CRAFT-R30** + **FIRE-BEST-AI** + **FIRE-PLACE-STICKY** + **CLOTHING-HAS-TAILOR** + **SMITH-LADDER-PEER-KIND** + **SMITH-CHISEL-PLAYER-CACHE** + **AI-JOB-SMITH-RESID** + **SHORTCRAFT-MAX-NEW-ACTOR DONE.** **AI-HANDLE-DEATH** + **AI-REMOVE-CONTAINER** + **AI-HANDLE-TEMP** + **AI-JOB-HUNT** + **AI-JOB-LUMBER** + **AI-JOB-COLLECT** + **AI-JOB-WATER** + **AI-JOB-FOODSERVER** + **AI-JOB-TAILOR** + **AI-FEED-MID** + **FILL-BUCKET** + **YOU-ARE-PROF** + **DO-WATERING-LOW** + **DO-CARROT-LOW** + **FILL-BEAN-BOWL** + **FILL-BEAN-HELD** + **FILL-BERRY-HELD** + **PULL-CARROT-ROW** + **DIE-SCORE-SKIP DONE.** Next **SPAWN-QUEUE-POLICY**. Skip unused `houseGeneration`.

**Loop:** token-free 60s `~\.grok\openlife-port-watchdog\watchdog-loop.ps1` (no Grok on skip). Resumes worker only if stuck and usage &lt; `resume_below_percent`. See [`QUEUE.md`](QUEUE.md).

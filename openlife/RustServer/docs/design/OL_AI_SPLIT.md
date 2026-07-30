# Split AI out of `ol-sim` (design)

**Status:** **Phase A–D + crate splits** (2026-07-30) —  
`PHASE_A_*` … `PHASE_D_MAIN_AI.md`, plus post-C splits for professions and pathing.

**Goal:** faster incremental builds + same write path as human clients + fast AI read path.

---

## Current crates (authoritative)

| Crate | Role | Approx. pure lines |
|-------|------|--------------------|
| **`ol-ai-api`** | `PlayerWriteInterface` / `PlayerReadInterface`, `FoodSearch`, DTOs; default food **r=40** | ~350 |
| **`ol-player-helper`** | Shared pure food scoring, eat gates, geom | ~1.3k |
| **`ol-ai-crafting`** | `craft_graph`, `craft_plan`, `craft_value` | ~1.1k |
| **`ol-ai-pathing`** | Path-reach / hostile / blockedByAI / sticky food fail marks | ~1.9k |
| **`ol-ai-helper`** | `Goal` / thin `pick_goal*`, `priority_ladder` (Haxe AI-PRIO) | ~3.0k |
| **`ol-ai-professions`** | Pure profession SMs (smith/farm/baker/potter/shepherd/fire) + `goal_expand` | ~16k |
| **`ol-main-ai`** | High-level `ThinkPlan` / `plan_hungry_food` over interfaces | ~260 |
| **`ol-ai`** | **Façade only** — stable `ol_ai::*` re-exports for server/sim | ~100 |
| **`ol-sim`** | Sole world writer; **adapters** + live sticky; still holds **duplicate** pure AI modules until re-export dedupe finishes | mega |
| **`ol-server`** | NPC schedule, `intent_tx`, thin MainAI apply | — |

### Graph

```text
ol-ai-api
ol-player-helper  → ol-ai-api
ol-ai-crafting    → ol-content
ol-ai-pathing     → (std only)
ol-ai-helper      → ol-ai-crafting, ol-ai-pathing, ol-content
ol-ai-professions → ol-ai-helper, ol-ai-crafting, ol-content
ol-ai             → api + helper + pathing + crafting + professions  (façade)
ol-main-ai        → ol-ai-api, ol-ai-helper, ol-player-helper
ol-sim            → ol-ai, ol-ai-api, ol-player-helper  (+ re-exports pathing/craft pure)
ol-server         → ol-sim, ol-ai, ol-main-ai, ol-net, …
```

Stable import for most code: **`ol_ai::*`** (façade).  
Direct crate deps preferred when editing one concern (e.g. path timers → `ol-ai-pathing` only).

---

## Goals vs priority ladder

| Layer | Types | Meaning |
|-------|--------|---------|
| **Goal** | `SeekFood`, `SeekObject`, `Explore`, `Idle`, `Flee`, `Hunt`, `Harvest` | Coarse action for write layer / self-play |
| **Profession** | Forager, Farmer, Smith, … | Biases job when nothing urgent wins |
| **PriorityRung** | Ordered slots of Haxe `doTimeStuffHelper` | Full decision ladder (escape → food → craft → job → idle) |
| **PriorityBand** | Blocked / Flee / Food / Feed / Craft / Job / Follow / Other | Log-friendly grouping of rungs |

Bridge: `goal_from_rung` / `pick_goal_from_ladder` map rungs → stable `Goal`s until every Haxe body is ported.

---

## Hard rules

1. AI **never** mutates `SimState` / `World` for gameplay.  
2. **Writes** = `PlayerWriteInterface` → same `NetIntent` as TCP clients.  
3. **Reads** = `PlayerReadInterface` / `WorldView` / `PlayerView` / `FoodSearch` (AI-fast, not TCP).  
4. Pure policy (goals, ladder, path marks *logic*, craft scores, profession SM transitions) lives in AI crates; **live** sticky fields and `apply_intent` side effects stay in `ol-sim`.

---

## Shared command interface

`ol_net::NetIntent` is the command payload:

| AI want | NetIntent |
|---------|-----------|
| Walk | `Move { … }` |
| Use tile / held | `Use { … }` |
| Drop | `Drop { … }` |
| SAY / JUMP / etc. | `Raw { tag, payload }` |

`PlayerWriteInterface` / `CommandSink` build those intents; NPC uses `NpcWriteTx`.

---

## Best food (default r=40)

- Constant: `ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS` = **40**  
- Pure scoring: `ol-player-helper` (`process_food` / `pick_best_search_food`)  
- Live: `ol-sim` `SimFoodSearch` / `search_best_food_full` + NPC `NpcNearbyFoodSearch`  
- MainAI: `plan_hungry_food` over `FoodSearch`

---

## Phase history

| Phase | Doc | Status |
|-------|-----|--------|
| A | `PHASE_A_PLAYER_INTERFACES.md` | Done — `ol-ai-api` write/read |
| B | `PHASE_B_PLAYER_HELPER.md` | Done — shared pure food |
| C | `PHASE_C_AI_HELPER_CRAFTING.md` | Done — crafting + helper; later split pathing + professions out of helper |
| C+ | pathing / professions crates | Done — `ol-ai-pathing`, `ol-ai-professions` |
| D | `PHASE_D_MAIN_AI.md` | Foundation — `ThinkPlan` / hungry food; ladder still mostly in `npc_ai` |
| E | (planned) | `ol-ai-llm` — provider + apply-plan → write interface only |
| F | (planned) | Finish **dedupe**: `ol-sim` pure modules → re-export AI crates; shrink `build.rs` AI patch surface |

---

## Dedupe status (`ol-sim` shadow modules)

| Module in sim | Extracted crate | Dedupe |
|---------------|-----------------|--------|
| `ai_path_reach.rs` | `ol-ai-pathing` | **done** — thin `pub use` re-export |
| `craft_graph.rs` / `craft_value.rs` | `ol-ai-crafting` | **done** — thin `pub use` re-export |
| `priority_ladder.rs` | (inside `ol-ai-helper`) | hash-identical; re-export next |
| `ai_goals.rs` | `ol-ai-helper` | **diverged** — merge carefully |
| `*_profession.rs` pure SMs | `ol-ai-professions` | **diverged** — live sticky still sim |
| `craft_topdown` / `get_or_craft` / `profession_scan` | still sim | later MainAI / scan crate |

---

## Compile model

| Edit | Rebuilds (target) |
|------|-------------------|
| Path fail timers / blockedByAI pure | `ol-ai-pathing` only |
| Priority ladder / escape sensors | `ol-ai-helper` |
| Craft value weights | `ol-ai-crafting` |
| Profession pure SM | `ol-ai-professions` |
| Write/read trait shape | `ol-ai-api` |
| Combat / USE transitions | `ol-sim` (not pure AI crates) |
| NPC schedule I/O | `ol-server` |

---

## Non-goals (first pass)

- Multi-server AI  
- Full LLM inside pure crates (keep provider / HTTP on server side initially)  
- One-shot rewrite of all professions  

---

## Success criteria

- [x] `ol-ai-api` write + read + food r=40  
- [x] NPC food seek via `FoodSearch` / shared pure scorer  
- [x] NPC physical actions through `NetIntent` / write interface  
- [x] Crafting / helper / pathing / professions as separate crates  
- [x] MainAI foundation (`plan_hungry_food`)  
- [ ] Changing pure profession file does **not** recompile full `ol-sim` lib body (needs sim re-export / drop of shadow copies)  
- [ ] Docs: this file + ARCHITECTURE_RUST + BUILD_SPEED current  

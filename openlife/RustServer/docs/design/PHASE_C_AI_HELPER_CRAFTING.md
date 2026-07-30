# Phase C — AiHelper + AiCraftingHelper (+ pathing / professions)

**Status:** done (2026-07-30); **post-C splits** pathing + professions also landed  
**Depends on:** Phase A + B  
**Git:** commit without push unless asked

## Crates (current)

| Your name | Crate | Contents |
|-----------|--------|----------|
| **AiCraftingHelper** | `ol-ai-crafting` | `craft_graph`, `craft_plan`, `craft_value` |
| **AiPathing** | `ol-ai-pathing` | `ai_path_reach` — not-reachable / hostile / blockedByAI / sticky fail marks |
| **AiHelper** | `ol-ai-helper` | `ai_goals` + `priority_ladder` (Goal / Profession / ladder sensors); re-exports pathing for stable paths |
| **AiProfessions** | `ol-ai-professions` | Pure profession SMs + `goal_expand` (smith craft pickers) |
| **façade** | `ol-ai` | Re-exports API + helper + pathing + crafting + professions (`ol_ai::*`) |

## Graph

```text
ol-ai-api
ol-player-helper → ol-ai-api
ol-ai-crafting → ol-content
ol-ai-pathing  → (std only)
ol-ai-helper → ol-ai-crafting, ol-ai-pathing, ol-content
ol-ai-professions → ol-ai-helper, ol-ai-crafting, ol-content
ol-ai → api + helper + pathing + crafting + professions  (façade only)
ol-sim / ol-server → ol-ai (stable import path)
```

## Compile benefit

| Edit | Rebuilds |
|------|----------|
| Craft value weights | `ol-ai-crafting` (+ link dependents) |
| Path-reach / fail marks | `ol-ai-pathing` |
| Priority ladder / escape | `ol-ai-helper` |
| Profession pure SM | `ol-ai-professions` |
| Write/read trait shape | `ol-ai-api` |
| NPC scheduler I/O | `ol-server` only |

## Goals vs ladder (in helper)

- **`Goal` / `Profession` / thin `pick_goal*`** — self-play and action-layer labels  
- **`PriorityRung` / `PriorityBand` / sensors** — full Haxe `doTimeStuffHelper` order (`priority_ladder.rs`)  
- Profession **state machines** live in **`ol-ai-professions`** (helper does **not** depend on professions; `goal_expand` breaks the cycle)

Orchestration / live sticky state in `npc_ai` remains server-side until MainAI absorbs more rungs (Phase D).

## Next (Phase D / E / F)

- **MainAI** (`ol-main-ai`): expand `ThinkPlan`; shrink `npc_ai` to schedule + `apply_plan`  
- **AiLlmHandler** (`ol-ai-llm`): provider + apply-plan → write interface only  
- **Dedupe**: `ol-sim` shadow pure modules → `pub use` from AI crates (pathing/craft first when hashes match)

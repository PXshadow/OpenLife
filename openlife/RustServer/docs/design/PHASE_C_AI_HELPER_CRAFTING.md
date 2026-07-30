# Phase C — AiHelper + AiCraftingHelper

**Status:** done (2026-07-30)  
**Depends on:** Phase A + B  
**Git:** commit without push when ready

## Crates

| Your name | Crate | Contents |
|-----------|--------|----------|
| **AiCraftingHelper** | `ol-ai-crafting` | `craft_graph`, `craft_plan`, `craft_value` |
| **AiHelper** | `ol-ai-helper` | `ai_goals` + `priority_ladder`, `ai_path_reach`, `professions`, profession pure SMs |
| **façade** | `ol-ai` | Re-exports API + helper + crafting (stable `ol_ai::*` for server/sim) |

## Graph

```text
ol-ai-api
ol-player-helper → ol-ai-api
ol-ai-crafting → ol-content
ol-ai-helper → ol-ai-crafting, ol-content
ol-ai → ol-ai-api, ol-ai-helper, ol-ai-crafting  (façade only)
ol-sim / ol-server → ol-ai (unchanged import path)
```

## Compile benefit

| Edit | Rebuilds |
|------|----------|
| Craft value weights | `ol-ai-crafting` (+ link dependents) |
| Priority ladder / escape | `ol-ai-helper` |
| Write/read trait shape | `ol-ai-api` |
| NPC scheduler I/O | `ol-server` only |

## Note on profession SMs

Pure profession pickers live in **`ol-ai-helper`** because `ai_goals` calls them.
Orchestration / live sticky state in `npc_ai` remains server-side until **MainAI** (Phase D).

## Next (Phase D / E)

- **MainAI** (`ol-main-ai`): `think()` API over write/read interfaces; shrink `npc_ai` to schedule + send
- **AiLlmHandler** (`ol-ai-llm`): provider + apply-plan → write interface only
- Optional: dedupe `ol-sim` copies of goals/craft that still shadow helper crates

# Phase D — MainAI (`ol-main-ai`)

**Status:** foundation done (2026-07-30)  
**Default food radius:** **40** tiles (shared with player SearchBestFood)

## Role

High-level **think plans** that only use:

| Interface | Use |
|-----------|-----|
| `FoodSearch` / `PlayerReadInterface` | Best food, sensors |
| `PlayerWriteInterface` | MOVE / USE / DROP (same as humans) |

No direct world mutation.

## API

```rust
ThinkSensors { conn_id, x, y, food_store, food_store_max, held_id, moving }
ThinkPlan::Idle | SeekFood { tx, ty, food_id } | UseFoodTile { ... }

plan_hungry_food(food: &dyn FoodSearch, sensors) -> ThinkPlan
apply_plan(write: &mut impl PlayerWriteInterface, sensors, plan) -> bool
```

## Wiring today

- NPC sticky food adopt: `plan_hungry_food` + `NpcNearbyFoodSearch` (pure SearchBestFood scorer, r=40)
- Full profession ladder / multi-step path still in `npc_ai` (not fully moved)

## Next

- Expand `ThinkPlan` for craft / profession rungs
- Move more of the NPC ladder into `think()` and leave ol-server as schedule + `apply_plan` only
- Phase E: `ol-ai-llm` apply-plan → write interface

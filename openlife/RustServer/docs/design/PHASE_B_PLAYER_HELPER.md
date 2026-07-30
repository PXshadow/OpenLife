# Phase B — PlayerHelper (`ol-player-helper`)

**Status:** done (2026-07-30)  
**Depends on:** Phase A (`ol-ai-api`)  
**Not pushed** until you ask

## Goal

Shared **pure** logic for both AI and ordinary player/sim paths, so:

- AI food scoring and player eat gates use the same math
- Changing food scoring does not force rewriting live scan code
- Compile unit is smaller than mega-`ol-sim`

## Crate: `ol-player-helper`

| Module | Contents |
|--------|----------|
| `geom` | `in_count_close_square`, `calculate_distance_sq`, `chebyshev` |
| `food_eat_gates` | `can_eat_obj*`, yum/meh/super-meh, `starving_factor`, `YUM_BONUS`, … |
| `food_search` | Full pure SearchBestFood scoring (`process_food`, stock, danger, pick best) |

**Depends on:** `ol-ai-api` only (for default food radius constant / chebyshev).

**Does not depend on:** `ol-sim`, HTTP, net.

## Wiring

```text
ol-ai-api
    ↑
ol-player-helper
    ↑
ol-sim  (yum re-exports pure free fns; search_best_food.rs re-exports food_search)
    ↑
ol-server (NPC FoodSearch → SimFoodSearch / NpcNearbyFoodSearch)
```

- `ol-sim/src/search_best_food.rs` → `pub use ol_player_helper::food_search::*;`
- `ol-sim/src/yum.rs` → `pub use ol_player_helper::{ can_eat_*, is_obj_*, YUM_BONUS, … };` for pure free functions; `YumState` stays in sim
- `SimFoodSearch` builds pure `SearchFoodCand` from ground tiles and ranks with `pick_best_search_food`

## Best food (default r=30)

Still via `PlayerReadInterface` / `FoodSearch`:

- API default: `ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS` = **30**
- Sim adapter: `best_food_for_ai` / `SimPlayerRead::best_food_default`
- NPC thread: `NpcNearbyFoodSearch` (pre-scanned nearby) still r=30

## Next (Phase C)

- `ol-ai-helper` (goals, path-reach pure, ladder)
- `ol-ai-crafting` (craft graph/value/plan)
- Migrate more raw NPC `try_send` to `PlayerWriteInterface`

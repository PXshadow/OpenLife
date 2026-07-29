# EATEN-FOOD-PCT / world_food_map

**Date:** 2026-07-28  
**Mode:** implement  
**Haxe:** `WorldMap.eatenFoodPercentage` / `addFoodStatistic` / `getFoodFactor` / `getEatenFoodPercentage`; eat via `GlobalPlayerInstance.doEating`; horse via `TransitionHelper.doHorseStuffPossible` → `doEating`; AI via `AiHelper.SearchBestFood` / `processFood`.

## Status: DONE (gap-close of WORLD-FOOD-FACTOR residual)

### Already present (WORLD-FOOD-FACTOR + FOODSTATS-DISK)

- `ol-sim/world_food_stats.rs` — `WorldFoodStats` maps, `add_food_statistic`, live `%` recompute, HQ rollup, starving factor
- `try_eat_held` × world FoodFactor × starving + stats
- FEED / NURSE `feed_fill_with_world_factors` + stats
- `search_best_food_full` cand `food_factor: state.world_food.get_food_factor(food_id)`
- FoodStats.txt dump + HTML pure + ObjectCounts pure

### This chunk

1. **Horse mount-eat parity** — `try_horse_eat` multiplies `get_food_factor` × `get_starving_food_factor` and calls `add_food_statistic` (Haxe doHorse→doEating).
2. **Horse superMeh full trade** — age + prestige−1 or hits+1/age+0.8/woundedBy/food_max death (Haxe L3195–3206), matching `try_eat_held`.
3. **Docs** — matrix / TODO / QUEUE / CALL_INDEX mark **EATEN-FOOD-PCT** DONE.
4. **Intentional delta** — Rust recomputes `eatenFoodPercentage` on each `add_food_statistic` (Haxe only on `writeFoodStatistics` save). Live SearchBestFood / eat factors see session percentages immediately.

### Tests

- `world_food_stats::*` (pure bands / HQ / stats format)
- `try_eat_held_applies_world_food_factor_and_stats`
- `feed_other_applies_world_food_factor_and_stats`
- `horse_eat_applies_world_food_factor_and_stats`
- `horse_eat_super_meh_burns_prestige`
- `search_best_food::food_factor_changes_ranking`

### Residuals (out of scope)

- `FoodFactorEaten*` bands still ModuleConst (not LiveSettings) — C-SS long-tail
- Lineage last-day window for starving (session counters approximate)
- Full feed-other yum fill path
- content `foodFromTarget` / tryGoto / AI not-reachable set

### Apply

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer\crates\ol-sim
python _wire_build_eaten_food_pct.py
# or: python src\_apply_eaten_food_pct.py  (source+docs only)
# then: cargo test -p ol-sim --lib -- horse_eat_applies_world_food_factor
```

`build.rs` pure wire (`build_eaten_food_pct.rs`) also applies on next `cargo test -p ol-sim` once wired.

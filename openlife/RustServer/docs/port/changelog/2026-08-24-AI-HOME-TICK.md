# AI-HOME-TICK — searchNewHomeIfNeeded on think tick

## Haxe

- `AiBase.doTimeStuffHelper` ~599–603 — after `isMoving()` return: `searchNewHomeIfNeeded` then `foundFamily` then `allyUp`
- `AiBase.searchNewHomeIfNeeded` ~8174–8196 — always returns false (side-effect only)
- Stay if home is IsOven/753 and not hungry and pop ≤ `AIMigrateVillagePopulationSize` (10)
- Else `SearchNewHome`; assign only if `home.tx != newHome.tx && home.ty != newHome.ty`
- `CountPopulation(home, 2)` — AIs only; starving ×2; age in `[MinAgeToEat, MaxAge-2]`
- `SearchNewHome` swamp skip uses `getOriginalBiomeId`

## Rust

| Symbol | Role |
|--------|------|
| `tick_search_new_home_if_needed` | live AI think; before `tick_ai_ally_up`; skip moving |
| `should_assign_new_home` / `apply_new_home_if_needed` | Haxe AND-coords assign |
| `home_search_biome` / `home_search_oven_tuple` | originalBiome fallback to live |
| `collect_home_search_ovens` | global ovens else local r=80 |

## Tests

```
cargo test -p ol-sim --lib -- speech:: home_oven search_new_home count_home_population should_assign_new_home home_search_biome tick_search_new_home say_do_commands_home_bang
```

## Residual

1. ~~npc `foundFamily` not on think tick~~ **AI-FOUND-FAMILY**
2. ~~AI-SAY-HELPER HOME! still local r=40 in build wire~~ template r=80/global ovens; live fan-out still unwired

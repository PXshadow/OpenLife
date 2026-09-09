# AI-HOME-OVEN — Home = oven / town

## Haxe

- `ObjectData.IsOven` 237 / 247 / 249 / 250
- `AiHelper.SearchNewHome` ~2086–2116 — `WorldMap.ovens`; swamp no-floor skip; floor `quadDistance /= 2`; `bestDistance = 80²`
- `AiBase.searchNewHomeIfNeeded` ~8174 — stay if IsOven or Adobe Rubble 753 unless hungry or pop > `AIMigrateVillagePopulationSize` (10)
- `AiBase.CountPopulation` ~1263 — starving counted ×2
- `HOME!` / `setNewNome` assign `player.home` + AI followers

## Rust

| Symbol | Role |
|--------|------|
| `HOME_OVEN_IDS` 237/247/249/250 | Haxe IsOven (was 237/238/752/753) |
| `ADOBE_RUBBLE_HOME` 753 | still-home tile, not SearchNewHome candidate |
| `search_new_home` / `home_oven_scored_quad` / `home_oven_biome_allowed` | SearchNewHome scoring |
| `should_search_new_home` / `search_new_home_if_needed` / `count_home_population` | migrate gate + starving×2 |
| `apply_do_commands_live_ex(..., global_ovens)` | HOME! uses `world_map_time.ovens` when filled |

## Tests

```
cargo test -p ol-sim --lib -- speech:: home_oven search_new_home count_home_population say_do_commands_home_bang
```

## Residual

1. ~~npc `doTimeStuffHelper` `searchNewHomeIfNeeded` not on think tick~~ **AI-HOME-TICK**
2. ~~swamp uses live `get_biome` not originalBiome~~ `home_search_biome`
3. ~~AI-SAY-HELPER HOME! still local r=40 in build wire~~ template uses `collect_home_search_ovens` (r=80 / originalBiome); live `fan_out_ai_say_scripted` still not in `lib.rs`

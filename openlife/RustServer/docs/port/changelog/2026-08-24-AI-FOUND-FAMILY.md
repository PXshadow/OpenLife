# AI-FOUND-FAMILY — foundFamily on think tick

## Haxe

- `AiBase.doTimeStuffHelper` ~602 — after `searchNewHomeIfNeeded`, before `allyUp`
- `AiBase.foundFamily` ~8199–8237 — void (does not consume think)
- Gates: `myEveId == id`; same-account eve lineage; follow same `getColor`; prestige ≥ 50; same-family followers ≥ 4; coins ≥ 10; `GetFamilyNameFromList`
- Success: SAY `I AM {name}`; if following and rand ≤ 0.5 SAY `I FOLLOW ME`
- Name apply / coin spend / `myEveId` rewrite happen in `NamingHelper.DoNaming` on `I AM` (not in foundFamily itself)

## Rust

| Symbol | Role |
|--------|------|
| `plan_found_family` / `FoundFamilyPlan` | Haxe gates + say lines |
| `get_family_name_from_list` | unused name from curated `FAMILY_NAMES` |
| `LineageNode.my_eve_id` | session founder id (not OLN-persisted) |
| `tick_found_family` | live think; apply name + founder + coin cost + SAY |

## Tests

```
cargo test -p ol-sim --lib -- plan_found_family tick_found_family get_family_name_from_list
```

## Residual

1. ~~Full `DoNaming` I AM (follower eve migrate, NM fan-out)~~ **AI-NAMING-IAM DONE**
2. Full `lastNames.txt` 2-letter map + random second-char mutate
3. ~~AI-SAY-HELPER HOME! local r=40 in build wire~~ template uses `collect_home_search_ovens` (global else r=80 + originalBiome); live `fan_out_ai_say_scripted` still not in `lib.rs`

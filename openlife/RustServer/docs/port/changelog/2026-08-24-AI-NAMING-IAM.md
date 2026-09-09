# AI-NAMING-IAM — DoNaming I AM family

## Haxe

- `NamingHelper.DoNaming` ~40–173 — `I AM` without `!`; third token is family name
- Starting family `SNOW` skips founder gates (`foundNewFamily=false`)
- Else prestige/followers/coins / already-Eve rejects (private say)
- Unused name among living; foundNewFamily: coins, `myEveId`, migrate same-family followers (close relatives always)
- `foundFamily` only SAYS; DoNaming applies

## Rust

| Symbol | Role |
|--------|------|
| `plan_do_naming_iam` / `DoNamingIam` | I AM planner |
| `should_migrate_found_family_follower` | close-relative / rand gate |
| `apply_do_naming_iam_live` | SAY + `tick_found_family` apply |
| `STARTING_FAMILY_NAME` | SNOW |

## Tests

```
cargo test -p ol-sim --lib -- plan_do_naming_iam say_i_am tick_found_family get_family_name_from_list
```

## Residual

1. ~~YOU ARE first-name (**AI-NAMING-YOU-ARE**)~~ **DONE**
2. Happy emote on rename
3. lastNames.txt 2-letter map
4. Follower migrate `WorldMap.randomFloat` (same-home roll 1.5 residual)

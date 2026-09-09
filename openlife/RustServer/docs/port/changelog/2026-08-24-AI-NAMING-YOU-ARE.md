# AI-NAMING-YOU-ARE — DoNaming YOU ARE first-name

## Haxe

- `NamingHelper.DoNaming` ~45–198 — `YOU ARE`; target `heldPlayer` else `getClosestPlayer(5)`
- Only if `target.name == StartingName` (SPOON)
- `GetNameFromList(token, isFemale)`; set `target.name`; age>3 target says `{name} {family}`
- `sendNameToAll`; speaker happy emote

## Rust

| Symbol | Role |
|--------|------|
| `plan_do_naming_you_are` / `DoNamingYouAre` | YOU ARE planner |
| `pick_you_are_target` | held else closest Chebyshev ≤5 |
| `get_first_name_from_list` | curated FIRST_NAMES (gender split residual) |
| `apply_do_naming_you_are_live` | SAY apply + NM |
| `STARTING_NAME` | SPOON |

## Tests

```
cargo test -p ol-sim --lib -- plan_do_naming_you_are say_you_are say_i_am
```

## Residual

1. Gender-split male/female name files (`FIRST_NAMES` mixed)
2. ~~Happy emote on speaker~~ **AI-YOU-ARE-EMOTE DONE**
3. lastNames.txt 2-letter map
4. PathfinderNew 100ms timeout (**AI-PATHFINDER-TIMEOUT**)

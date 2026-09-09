# AI-ALLY-UP-HIRE — allyUp skip when hired

## Haxe

`AiBase.allyUp` ~8250: `if (player.hiredByPlayer != null) return;`
Then 10s cadence, age≥10, most powerful at home, SAY `I FOLLOW Name `.

## Rust

| Symbol | Role |
|--------|------|
| `should_skip_ally_up_if_hired` | hired_boss > 0 |
| `plan_ally_up` | Haxe gates + `I FOLLOW {name} ` |
| `pick_most_powerful_at_home` / `leadership_power` | GetMostPowerful lite (no familyPrestige) |
| `tick_ai_ally_up` | live SAY; hired skip |
| AutoFollow acquire | skip hired workers |

## Tests

```
cargo test -p ol-sim --lib -- ally_up pick_most_powerful
```

## Residual

familyPrestige in power; apply_do_commands on I FOLLOW say (currently SAY only).

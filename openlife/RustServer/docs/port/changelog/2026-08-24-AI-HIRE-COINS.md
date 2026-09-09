# AI-HIRE-COINS — hire NEED coins + countHiredPeople age

## Haxe

- `processHireCommand` ~2383–2390: `missing = ceil(needed - coins)`; `NEED one more coin!` / `NEED n more coins!`; `YOU ARE TOO POOR!`; `doEmote(sad)`
- `countHiredPeople` ~2425: skip deleted and `age > 55`

## Rust

| Symbol | Role |
|--------|------|
| `hire_need_coins_say` / `HIRE_TOO_POOR_SAY` | spoken NEED / TOO POOR |
| `HireFail::NeedCoins { missing }` | try_hire poor-hirer path |
| `SocialState::count_hired` | skip `age > 55` |

I HIRE coin transfer / class / color / follow was already **FOLLOW-HIRE-DELAY** DONE.

## Tests

```
cargo test -p ol-sim --lib -- hire_ need_coins count_hired
```

## Residual

1. npc `allyUp` skip when `hiredByPlayer` set (allyUp not ported)
2. Human-target spoken `Its a human!` still private_ps only

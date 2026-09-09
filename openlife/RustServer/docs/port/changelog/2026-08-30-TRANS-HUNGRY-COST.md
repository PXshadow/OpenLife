# TRANS-HUNGRY-COST — Transition.hungryWorkCost PatchTransitions table

## Haxe

- `TransitionData.hungryWorkCost` default 0; `hungryWorkTemperature` default −1 (not in transition files).
- Set only by `ServerSettings.PatchTransitions` (`getTransition` / `new TransitionData` / Property Gate 2962 loop).

## Rust

| Symbol | Role |
|--------|------|
| `Transition.hungry_work_cost` / `hungry_work_temperature` | defaults 0 / −1; not OLT1 |
| `apply_default_hungry_work_cost_patches` / `_ex` | text load + `finish_cache_boot`; knob for 502+408 |
| `patch_hungry_work_cost_by_target` | Property Gate: skip actor 0; 0.1; non-skewer (not 139/852) → 5 |
| `content_pair_hungry_work_cost` | sums `tr.hungry_work_cost` |
| `apply_use_at` | reads `tr_work.hungry_work_cost` / `hungry_work_temperature` |

Commented-out Haxe rows omitted. `hungryWorkTemperature` stays −1.

## Tests

```
cargo test -p ol-content --lib -- hungry
cargo test -p ol-ai-professions --lib -- hungry
cargo test -p ol-sim --lib -- hungry_work -- --test-threads=1
```

## Residual

~~hungry-work emote/FX `sendFoodUpdate`~~ **AI-HUNGRY-EMOTE DONE**; ~~npc `craftingTasks` drain~~ **AI-CRAFT-TASKS-DRAIN DONE**; ~~DeferPottery staging~~ SeekOrCraft kiln/plate.

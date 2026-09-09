# AI-HUNGRY-COST — checkHungryWorkCostById pair lookup

## Haxe

- `AiBase.checkHungryWorkCostById` ~1412–1450
- `shortCraftOnTarget` ~2728 always-on food gate
- `TransitionData.totalHungryWorkCost` = `actor.hungryWork + newTarget.hungryWork + trans.hungryWorkCost`
- `ObjectData.hungryWork` from `ServerSettings.PatchObjectData`:
  - description `+hungryWork` → `HungryWorkCost` (LiveSettings default 5)
  - loose fence 1845/1846/1847 → 5
  - tools 34/334/502 → `1 * HungryWorkToolCostFactor` (default **0**; Haxe block commented)
  - 3146 / 1853 → HungryWorkCost knob
  - extra PatchObjectData ids (hoe, oven, ponds, …) for Haxe parity
- `trans.hungryWorkCost` mostly 0 (PatchTransitions residual)

## Rust

| Symbol | Role |
|--------|------|
| `object_hungry_work(id, description, knob)` | description + id table (no ObjectDef field) |
| `total_hungry_work_cost` | sum actor + new_target + trans |
| `content_pair_hungry_work_cost` | primary / last-use / ground `(actor,-1)`; new_target from trans; `tr.hungry_work_cost` |
| `scan_held_hungry_work_cost` | live scan-wide `(held_id, -1)` |
| `apply_profession_ladder_tick` / `apply_profession_scan_tick` | `state.gameplay.hungry_work_cost` |
| npc `ProfessionScanInput` | `NpcConfig.hungry_work_cost` from LiveSettings (default 5) |
| `check_hungry_work_cost_by_id` | refuse when `food < cost + 1` and cost > 0 |

Farm/shepherd/baker/smith shortCraft copies use per-target pair when `ProfessionScanInput.content` is set (`2026-08-30-AI-SHORTCRAFT-PAIR.md`).

## Tests

```
cargo test -p ol-ai-professions --lib -- hungry
cargo test -p ol-sim --lib -- hungry_work short_craft_apply -- --test-threads=1
```

## Residual

1. ~~Per-target actor+target lookup at shortCraft helper sites~~ **DONE** (`2026-08-30-AI-SHORTCRAFT-PAIR.md`)
2. ~~`Transition.hungryWorkCost` / `hungryWorkTemperature` content fields~~ **DONE** (`2026-08-30-TRANS-HUNGRY-COST.md`)
3. npc `craftingTasks` drain / job Defer*
4. ~~hungry-work emote/FX sendFoodUpdate~~ **AI-HUNGRY-EMOTE DONE**

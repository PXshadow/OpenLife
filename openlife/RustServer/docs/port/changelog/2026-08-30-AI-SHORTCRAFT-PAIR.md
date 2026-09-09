# AI-SHORTCRAFT-PAIR — per-target shortCraft hungry cost

## Haxe

`AiBase.shortCraftOnTarget` always calls `checkHungryWorkCostById(actor, target)` for that pair (`GetTransition(actor,target)` then `(actor,-1)`).

## Rust

| Symbol | Role |
|--------|------|
| `short_craft_pair_hungry_cost` | `ProfessionScanInput.content` → `content_pair_hungry_work_cost(actor,target,knob)`; else scan-wide `transition_hungry_cost` |
| `ProfessionScanInput.content` / `hungry_work_cost_knob` | live `Arc<ContentDb>` + `HungryWorkCost` knob; tests leave `None` |
| farm / shepherd / baker / smith / fire-food ShortCraft | pair cost at conversion, not held `(held,-1)` |
| `scan_held_hungry_work_cost` | unchanged scan-wide fill |

## Tests

```
cargo test -p ol-ai-professions --lib -- hungry
cargo test -p ol-sim --lib -- hungry_work short_craft_apply farm_action_to_live baker_ smith_defer farm_short_craft_pair baker_short_craft_pair smith_short_craft_pair -- --test-threads=1
```

Held pair cost 0 + `(actor,target)` cost > 0 + `food < cost+1` → `RefuseHungry`. High food → not refuse. No ContentDb → `inp.transition_hungry_cost`.

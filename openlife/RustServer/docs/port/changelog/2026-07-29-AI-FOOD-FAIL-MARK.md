# AI-FOOD-FAIL-MARK / food_use_fail_30s

## Haxe
- `AiBase.isPickingupFood` ~8694–8700: `done == false` → `addNotReachableObject(food, 30)`; `foodTarget = null`
- No `didNotReachFood++` on action fail (that is goto/path fail only)

## Rust
| Behavior | Surface |
|----------|---------|
| 30s not_reachable | `NOT_REACHABLE_FOOD_SECS` / `mark_food_path_fail` |
| Prefer food 30s vs age-gate USE | `mark_use_or_food_path_fail` |
| Live Player maps | `mark_path_fail_after_use_live` (AI-only) |
| shortCraft USE fail | `apply_short_craft_live_intent` + profession_scan |
| NetIntent USE fail | after `try_eat_held` fails (not before) |
| npc async settle | `pending_food_xy` + `settle_pending_food_use_fail` |

## Tests
- `food_action_fail_30s_clears_sticky`
- `mark_use_or_food_path_fail_prefers_food_30s`
- `settle_pending_food_use_fail_marks_30s`
- `mark_path_fail_after_use_live_food_30s`

## Residual
- Player sticky sync from npc path maps (AI-GOTO-FOOD)
- remove-from-container fail mark polish (PATH-REACH)

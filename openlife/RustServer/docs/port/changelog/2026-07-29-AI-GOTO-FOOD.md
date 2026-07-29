# AI-GOTO-FOOD / food_explore_goto (2026-07-29)

## Summary

Close Haxe food/explore Goto gaps: sticky `foodTarget`, `gotoObj` receding, dual-pass animal path marks on SeekFood/Explore/takeover/selfplay.

## Port map

| Haxe | Rust |
|------|------|
| `gotoObj` receding + lastGotoObj | `plan_goto_obj` / `LastGotoObj` / Player `ai_last_goto_obj_*` |
| `gotoAdv` dual-pass | existing `goto_path_outcome` + npc food walk mark |
| `foodTarget` sticky / clear on fail | `StickyFoodTarget` / `resolve_sticky_food` / `NpcFoodGotoState` |
| `didNotReachFood` gate + increment | `ai_did_not_reach_food` + `apply_food_goto_fail` |
| `resetTargets` on goto fail | `apply_food_goto_fail` → `clear_action_targets` |
| `isPickingupFood` walk | npc SeekFood sticky + dual-pass (lite) |
| explore Goto animals | Explore / takeover: `is_walkable_with_animals` |
| selfplay SeekFood plain path | `next_step_consider_animals` |

## Tests

- `ai_path_reach::plan_goto_obj_receding_vs_proceed`
- `ai_path_reach::sticky_food_resolve_and_fail_effects`
- existing pathfind dual-pass corridor / footprint

## Residual

- Full isPickingupFood SM polish (→ AI-PICKUP-FOOD / remaining dropHeld edge)
- ~~Live USE fail → `mark_food_path_fail` 30s~~ → **AI-FOOD-FAIL-MARK DONE**
- Sync npc sticky maps onto Player.ai_path_reach / ai_block_targets.food_target
- GotoHelper 5-end-tile tweak
- Full isAnimalNotDeadlyForMe biome/hits/weapon

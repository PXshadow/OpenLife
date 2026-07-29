# DROP-HELD-LIVE / drop_held_enqueue

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** PARTIAL — live enqueue wired; table/small-food prefer residual

## Haxe

- `AiBase.dropHeldObject` ~5267–5649 (live USE/DROP/goto/self)
- `storeInQuiver` → `self(0,0,5)`
- `considerDropHeldObject` UseUpDough ~5203 before fire/oven/forge
- pottery `dropHeldObject(allowAllPiles)` gather clay

## Rust

- `drop_held_ai.rs`: `plan_drop_held_live`, `drop_held_input_from_sensors`, `smart_drop_held_from_sensors`, `consider_drop_held_decision_ex`, `self_clothing_raw_payload`
- `short_craft_intent.rs`: `ShortCraftLiveIntent::SelfClothing`, `smart_drop_held_profession` (parent bridge)
- `profession_scan.rs`: pottery/farm/smith/baker DropHeld → smart planner
- `npc_ai.rs`: Goto Move, SelfClothing Raw SELF, force_drop_at_feet smart drop
- `selfplay.rs`: SMART-DROP / USE / SELF / GOTO

## Tests

- `drop_held_ai::*` (31) — prior 26 + UseUpDough consider, plan resolve, clay basket live, SelfClothing live, banana feet live

## Residuals

- AiHelper ShouldDropOnTable / isSmallFoodToStore container prefer
- Live quiver clothing ids from full Player clothing snapshot (npc has no clothing on PlayerSnapshot)
- PreferShortCraft unresolved + BusyMoving wait tick
- Nested held_contains_clay from basket contents

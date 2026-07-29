# DROP-HELD-AI / drop_held_smart

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** PARTIAL — pure planner gap-close (container/pile/quiver/clay/Goto/185→85)

## Haxe

- `openlife/auto/AiBase.hx` `dropHeldObject` ~5267–5649
- `storeInQuiver` ~5087–5138
- `UseUpDough` ~5237–5261
- `considerDropHeldObject` ~5191–5234
- id tables `dropNearFire/Oven/Forge/WellItemIds` ~5164–5189
- `ObjectHelper.canAddToQuiver` ~767
- empty-search container `numSlots>0` → `useIsDropInContainer`

## Rust

- `crates/ol-sim/src/drop_held_ai.rs` — pure planner
- `crates/ol-sim/src/profession_scan.rs` — `ScanTile` num_slots / num_uses / contains_id / contained_count
- `crates/ol-sim/src/short_craft_intent.rs` — `ShortCraftLiveIntent::Goto`
- Wired as `short_craft_intent::drop_held_ai` via `#[path]` (build `build_craft_live_tick.rs`)

## Focus / gap-close this chunk

- Container free-slot drop-in (`UseAsDrop` when `num_slots>0`)
- Clay far-from-kiln prefers basket containing 126
- Pile full via `uses >= num_uses` (not uses≥10)
- `can_add_to_quiver(uses, num_uses)` capacity
- consider + special path PreferShortCraft 185→85
- `DropHeldDecision::Goto` → `ShortCraftLiveIntent::Goto` (walk, not DropAt)
- Forge flat-rock/stone `min_distance=-1` search rings
- Dry bean pod always bowl-fill; gooseberry second-closest after full first
- `fill_anchors_from_scan` kiln 283/642/238

## Tests

- `drop_held_ai::*` (26) — prior + container, clay basket, pile num_uses, quiver filled, 185→85, Goto map, forge min_d, bean near-home, second bowl

## Residuals

- Live npc_ai / selfplay enqueue of DropHeldDecision
- SELF clothing apply for quiver (slot 5)
- Table / small-food container prefer factors (Haxe ShouldDropOnTable / isSmallFoodToStore)
- useHeldObjOnTarget multi-step staging residual (CRAFT-LIVE-IO)

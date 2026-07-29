# BLOCKED-BY-AI / blocked_rebuild (2026-07-28)

## Summary

Live Haxe `AiBase.CalculateBlockedByAi` — wipe and rebuild global `blockedByAI` each tick from sticky AI food/use/drop targets and human `blockTargetForAi`.

## Status: **DONE** (core live wire)

## Files

- **Core pure:** `crates/ol-sim/src/ai_path_reach.rs` — `AiStickyBlockTargets`, `rebuild_blocked_by_ai_from_sticky`, `should_set_block_target_for_ai`, `block_claim_number_of_uses`
- **Player:** `Player.ai_block_targets`
- **Live:** `rebuild_blocked_by_ai_live` + `note_ai_block_targets_from_live_intent` in `lib.rs`; **tick_vitals** calls rebuild (not decay-only)
- **Intent:** `apply_short_craft_live_intent` notes use/drop/food sticky before USE/DROP
- **USE:** `use_transition::apply_use_at` sets human/smith hammer `blockTargetForAi` (`player_block`)

## Behavior

1. AI shortCraft UseAt/DropAt → sticky claim on Player (food_value>0 → Food kind)
2. tick_vitals wipe+rebuilds `SimState.blocked_by_ai` from all living sticky rows
3. Human USE (or smith hammer 441) sets timed `player_block` (20s age gate on human loop)
4. AI agent chain: `player_block` mirrors into `ai_block_target` (no second age gate) → early-stops food/drop/use
5. Wound gate: Haxe `isWounded` = held is wound and ≠ `hiddenWound` (not mere `hidden_wound.is_some()`)
6. Instance `numberOfUses` only — no `ObjectData.num_uses` fallback

## Residual

- `removeFromContainerTarget` sticky never noted from live path
- Non-shortCraft AI job food/use/drop sticky sources incomplete
- `clear_action_targets` unused when AI finishes/switches jobs (stale claims)
- Haxe `RemoveBlockedByAi` mid-frame temporary unclaim not ported
- Animal-aware Goto / food-pickup 30s → **PATH-REACH** residual

## Tests

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- ai_path_reach sticky rebuild_from_sticky should_set_block block_claim player_block clear_action wounded_agent
```

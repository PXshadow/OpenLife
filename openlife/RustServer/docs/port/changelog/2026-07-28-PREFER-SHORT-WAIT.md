# PREFER-SHORT-WAIT / prefer_short_busy

**Date:** 2026-07-28  
**Mode:** implement  
**Status:** DONE (pure + live wire + tests)

## Haxe

- `AiBase.dropHeldObject` ~5537–5538: `if (myPlayer.isMoving()) return true` under `dropOnStart`
- Prefer-shortCraft specials before spatial drop (mutton oven/coals, soil bush/row, hoe, dough plate, rabbit 185→85)
- `shortCraftOnTarget` craftActor / maxNewActor; held==actor → USE
- Returning `true` while moving holds the AI tick (no fallthrough)
- `GetOrCraftItem` ~6152: `if (myPlayer.isMoving()) return true`

## Rust

- `ShortCraftLiveIntent::Wait` — hold-tick (no wire, no feet-drop fallback)
- `DropHeldDecision::to_live_intent`: `BusyMoving` → `Wait` (was `None`)
- Unresolved `PreferShortCraft` → `SeekOrCraft { craft_if_needed: craft_actor }` (was always false)
- `plan_drop_held_live` still resolves PreferShortCraft → UseAt when target in scan
- `smart_drop_held_profession_ex(..., is_moving)` + `ProfessionScanInput.is_moving`
- Profession DropHeld paths (farm/smith/baker/pottery) call `_ex` with `inp.is_moving`
- Ladder: Wait is terminal (no makeStuff fallthrough)
- GetOrCraft BusyMoving → Wait
- npc_ai: `is_moving` field + Wait arms (`prof_wait_busy_moving` / smart_drop_wait)
- selfplay: `SMART-DROP-WAIT` (no feet-drop fallback)
- lib reexports: `live_intent_is_wait`, `smart_drop_held_profession_ex`
- Free-function mapper: `prefer_short_busy_to_live.inc.rs` / `drop_held_decision_to_live_intent`

## Tests

- `busy_moving_to_wait_live_intent` (drop_held + get_or_craft)
- `drop_on_start_while_moving_is_busy_wait`
- `prefer_short_craft_uses_craft_actor_flag`
- `plan_resolves_prefer_short_before_live`
- `smart_drop_held_profession_ex_busy_moving_wait`
- `ladder_wait_terminal_helpers`

## Residuals

- maxNewActor count gate on PreferShortCraft still needs transition newActor id from content (stored on decision, not enforced in pure dropHeld)
- selfplay quiver default-empty (DROP-HELD-LIVE residual; npc force_drop fills)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- busy_moving -- --test-threads=1
cargo test -p ol-sim --lib -- prefer_short -- --test-threads=1
cargo test -p ol-sim --lib -- drop_on_start_while -- --test-threads=1
```

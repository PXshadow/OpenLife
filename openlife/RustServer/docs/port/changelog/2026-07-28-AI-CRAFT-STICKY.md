# AI-CRAFT-STICKY / craft_runtime (2026-07-28)

## Status: DONE

### Implemented

| Piece | Module | Notes |
|-------|--------|-------|
| `PlayerCraftAi` | `ol-sim/src/craft_ai_sticky.rs` | sticky shell: CraftAiRuntime + itemToCraftId + craftingTasks + itemToCraftName |
| `Player.craft_ai` | `player.rs` | survives ticks; birth wipe; begin_tick guard |
| `prepare_for_product` | same | product-change interrupt re-queue **+** `reset_for_product` (no stale trans) |
| `note_successful_use` | same | Haxe USE-done countTransitionsDone + countDone when product held/ground |
| `do_make_craft_command` | same | Haxe doMakeCraftCommand id + name + "Making …" say |
| `select_sticky_craft_for_tick` | same | begin_tick + continue unfinished / shift craftingTasks |
| `sticky_craft_sensor_flags` | same | CraftQueue ladder sensors |
| `apply_sticky_craft_queue_tick` | `profession_scan.rs` | live CraftQueue expand sticky + apply |
| Revive wipe | `spawn_player_inner` | `wipe_craft_on_birth` on deleted→alive |
| USE wire | `apply_use_at` | note_successful_use when sticky product active |
| AI entry | `apply_profession_scan_from_sensors` | begin_tick + sticky flags → CraftQueue |

### Tests

- `craft_ai_sticky::*` — 16 tests (newborn, re-queue, cooldown, countDone, MAKE, select, sensors)
- `player::craft_ai_sticky_on_player_defaults_and_survives`

### Residual

1. npc_ai path does not yet call `craft_item_with_player_craft_ai` / player sticky expand directly (profession scan CraftQueue covers sim entry)
2. pile_id_for still 0 on sticky queue tick (AI-CRAFT-MULTI)
3. Finished/Failed/Making say not pushed to speech wire (pure strings returned only)
4. Top-down DoTransitionSearch filters → AI-CRAFT-MULTI
5. wipe_on_birth clears craftingTasks (stricter than Haxe newBorn — intentional)

### Haxe anchors

- `AiBase.itemToCraft` / `failedCraftings` / `itemToCraftId` / `craftingTasks` / `itemToCraftName`
- `AiBase.newBorn` ~327–345
- `AiBase.doTimeStuffHelper` calledCraftItem + sticky continue + tasks ~435, ~667–680
- `AiBase.addTask` ~5656
- `AiBase.craftItemHelper` product change re-queue ~6678–6690
- `AiBase` USE done countDone ~9077–9089
- `AiBase.doMakeCraftCommand` ~8339

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- craft_ai_sticky
cargo test -p ol-sim --lib -- craft_ai_sticky_on_player
```

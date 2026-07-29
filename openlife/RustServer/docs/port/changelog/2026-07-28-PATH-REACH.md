# PATH-REACH / not_reachable_maps (2026-07-28)

## Summary

Live Haxe `AiBase` path-block maps for AI profession scans and food search.

| Haxe | Rust | Default TTL |
|------|------|-------------|
| `notReachableObjects` | `Player.ai_path_reach.not_reachable` | 90s |
| `objectsWithHostilePath` | `Player.ai_path_reach.hostile_path` | 20s |
| static `blockedByAI` | `SimState.blocked_by_ai` | 5s (rebuild or decay) |
| food fail override | `NOT_REACHABLE_FOOD_SECS` | 30s |

## Files

- **Core:** `crates/ol-sim/src/ai_path_reach.rs` — timed maps + pure CalculateBlockedByAi
- **Wire:** `Player.ai_path_reach`, `SimState.blocked_by_ai`, tick_vitals cleanup
- **Apply:** `profession_scan` tile filter + age-gated USE fail mark
- **Food:** `search_best_food_live.inc.rs` not_reachable + hostile_path from maps
- **NPC:** `ol-server/npc_ai.rs` per-NPC `path_reach` filter + walk-fail mark

## Behavior

1. `tick_vitals` decays all maps each frame (Haxe `cleanupBlockedObjects`)
2. Pure `calculate_blocked_by_ai` rebuild with DontBlockByAi / multi-use / animal / held-same-newTarget filters
3. Profession apply filters scan tiles; failed USE → age>3 notReachable else hostile
4. `search_best_food_full` skips personal not-reachable + blockedByAI; hostile tiles feed danger
5. npc_ai filters profession scan; walk-path fail → addNotReachable

## Residual

- ~~Live CalculateBlockedByAi rebuild~~ → **BLOCKED-BY-AI DONE** (tick_vitals wipe+rebuild + USE player_block + shortCraft sticky)
- ~~Animal-aware Goto fail (`blockedByAnimal` path recheck)~~ → **AI-ANIMAL-GOTO core** (dual-pass + footprints + npc walk mark)
- ~~Food-pickup fail 30s live wire~~ → **AI-FOOD-FAIL-MARK DONE**
- remove-from-container / Drop fail marks
- craft object scan `isObjectNotReachable` on multi-step get_or_craft

## Tests

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- ai_path_reach path_reach not_reachable
```

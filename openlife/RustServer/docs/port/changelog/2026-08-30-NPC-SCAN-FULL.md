# NPC-SCAN-FULL — live blockedByAI share + dropHeld nest

## Haxe

- `AiBase.isObjectNotReachable` ORs personal `notReachableObjects` with static `blockedByAI`
- `CalculateBlockedByAi` ~222 wipe+rebuild each AI frame
- `AddObjBlockedByAi` default **5s**
- `dropHeldObject` / `gatherClay`: `heldObject.containedObjects.length` + `heldObject.contains([126])` (Basket 292 + Clay 126)

## Rust

| Symbol | Role |
|--------|------|
| `merge_blocked_by_ai_max` | per-tile max remaining time |
| `blocked_by_ai_with_peer_progress` | clone global + insert peer tiles at `BLOCKED_BY_AI_DEFAULT_SECS` (5.0) |
| `BlockedByAiShare` / `new_blocked_by_ai_share` | `Arc<RwLock<HashMap<(i32,i32),f32>>>` |
| `mirror_blocked_by_ai_share` / `snapshot_blocked_by_ai_share` | sim write / npc read (poison → empty) |
| `SimBootLive.blocked_by_ai_share` | boot package |
| `SimState.blocked_by_ai_share` | attached at sim boot |
| `rebuild_blocked_by_ai_live` | after sticky rebuild, mirrors into the share |
| `npc_merged_blocked_by_ai` | share snapshot + other-conn `craft_progress` |
| npc think 2a2 / 2a3 / 2b / enqueue | one helper; personal `path_reach` filters kept; `target_reachable: true` unchanged |
| `PlayerSnapshot.held_contained` / `held_contains_clay` | nest length + clay-in-basket / bare 126 |
| npc `ProfessionScanInput` + `DropHeldSensorExtras` | snapshot nest (food pickup, force_drop 2c, quiver already wired) |

## Tests

- `merge_blocked_by_ai_max_keeps_larger_time` / `blocked_by_ai_with_peer_progress_inserts_default_and_max` / `blocked_by_ai_share_roundtrip`
- `snapshot_held_contained_and_clay_from_nest`
- `npc_merged_blocked_by_ai_uses_share_and_peer_craft`

```
cargo test -p ol-ai-pathing --lib -- blocked_by_ai
cargo test -p ol-sim --lib -- snapshot_held
cargo test -p ol-server --lib -- npc
cargo check -p ol-server
```

## Residual

- ~~npc `ignored_floor_ids` empty~~ **NPC-IGNORED-FLOOR DONE**
- BLOCKED-BY-AI: removeFromContainer sticky; clear_action on job switch

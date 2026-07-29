# Birth-relative client coordinates + MOVE path deltas

## Rules (vanilla OHOL / Haxe Open Life)

1. **Birth origin** (`Player.birth_x/y`, Haxe `gx/gy`, vanilla `birthPos`) is set once per life:
   - **Eve / wild spawn:** absolute spawn tile.
   - **Baby:** mother’s absolute tile at birth.
2. **Everything the client sends** with map coordinates is **relative to birth**:
   - `world = client + birth`
   - Applied to MOVE start, USE/DROP targets, etc.
3. **Everything the server sends** with map coordinates to a viewer is **relative to that viewer’s birth**:
   - `client = world - viewer.birth`
   - PU, PM, MC header, MX positions for that connection.
4. **MOVE path deltas** (protocol.txt / LivingLifePage): each `(xdelt, ydelt)` is relative to path **start** `(xs,ys)`, not to the previous step.

### Example

```text
birth = (488, 488)
client MOVE 0 0 @2 1 0 2 0#
→ world start (488, 488)
→ waypoints (489, 488), (490, 488)
→ steps (1,0), (1,0)
→ end (490, 488)   # not (491, 488)
```

## Code map

| Concern | Location |
|---------|----------|
| Birth fields + convert helpers | `ol-sim/src/player.rs` |
| Spawn Eve / mother birth set | `spawn_player`, `spawn_child` in `ol-sim/src/lib.rs` |
| Path delta conversion | `client_path_deltas_to_steps` / `truncate_walkable` in `move_path.rs` |
| Inbound MOVE/USE/DROP | `NetIntent` handlers in `lib.rs` |
| Outbound PM/PU/MC relative | path start PM, force PU, path-done PU, login bootstrap, `force_send_map_chunk` |

## Restart required

Rebuild and restart `ol-server` for clients to see birth-relative bootstrap (PU/MC at ~0,0).

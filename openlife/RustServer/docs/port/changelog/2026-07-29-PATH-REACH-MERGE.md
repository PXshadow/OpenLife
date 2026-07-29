# PATH-REACH-MERGE / dual_map_merge (2026-07-29)

## Summary

Live dual-map ownership bridge for Haxe single `AiBase` path maps.

| Side | Surface | Marks |
|------|---------|-------|
| Sim / NetIntent | `Player.ai_path_reach` | USE/DROP/REMV fail |
| NPC scheduler | `NpcProfessionState.path_reach` | walk fail, food settle, profession goto |

## Wire

1. `PlayerSnapshot.ai_path_reach` (serde skip) — NPC pull source
2. `npc_ai` each think: **pull once at think start** → cleanup → act (all arms) → `push_npc_path_reach_to_views`
3. **AI-TAKEOVER**: same pull-once + **push** after food/explore (was missing push)
4. `tick_vitals`: `merge_npc_path_reach_from_views` then personal `cleanup`
5. `publish_player_view` / `publish_all_player_views`: `preserve_view_path_reach_on_publish` max-merge prior view maps (no clobber of unabsorbed NPC marks)
6. Pure: `merge_path_reach_maps` + `sync_path_reach_bidirectional` + `preserve_view_path_reach_on_publish`

## Gap-close (same day)

| Gap | Fix |
|-----|-----|
| AI-TAKEOVER never push | push after each takeover think that pulled |
| publish clobber race | preserve max-merge on publish_player_view / publish_all |
| pull only food/profession | pull once per native + takeover think (covers explore/craft) |

## Tests

```powershell
cargo test -p ol-sim --lib -- path_reach
```

Focused: `path_reach_merge_*` / `path_reach_publish_*` / `preserve_view_path_reach_*` / `merge_path_reach_maps_*` / `sync_path_reach_*` / `player_snapshot_includes_path*`

## Residual

- dual independent cleanup (Player `cleanup(dt)` + NPC `cleanup(0.2*think_period)`) vs Haxe single `cleanupBlockedObjects(reactionTime)` — max-merge can slightly overshoot TTL
- empty-hand DROP pickup `apply_drop` (AI-PICKUP-FOOD)
- sticky foodTarget sync residual (AI-GOTO-FOOD)

# AI-CRAFT-NPC-ENQUEUE / npc_craft_enqueue (2026-07-29)

## Status: DONE (npc multi-step GetOrCraft enqueue + gap-close²)

### Closed residual

Wire profession `SeekOrCraft` / `CraftItem` staging through multi-step **GetOrCraftItem → craftItem** expand with **CraftScanFilters** (hostile / notReachable / blockedByAI), then enqueue USE/DROP/MOVE on the NPC action path.

**Gap-close² (this session):** live pile_id, ignoreFullPiles multi-use tiles, ScanTile.num_slots, peer blockedByAI merge.

| Piece | Module | Notes |
|-------|--------|-------|
| `resolve_seek_or_craft_live_ex_scan` | `get_or_craft_resolve.inc.rs` | multi-step + scan + is_moving Wait |
| `npc_enqueue_get_or_craft` / `_ex` | same | pure NPC helper; `_ex` = pile_id_for + full_pile_tiles |
| `get_pile_obj_id` / `pile_obj_id_from_content` | `get_or_craft.rs` | Haxe ObjectData.getPileObjId pure + ContentDb |
| `full_pile_tiles_from_scan` | `profession_scan.rs` | is_full_uses → CraftScanFilters.with_full_piles |
| `get_or_craft_objs_from_scan` | same | prefers ScanTile.num_slots (ObjectDef at scan) |
| `NpcProfessionState.craft_rt` | `ol-server/src/npc_ai.rs` | sticky failedCraftings / itemToCraft |
| craft_graph → `run_npc_scheduler` | `main.rs` + `npc_ai.rs` | reverse graph for expand |
| SeekOrCraft/CraftItem arm | `npc_ai.rs` | expand → USE/DROP/MOVE (`prof_goc_*`); peer craft_progress → blockedByAI |

### Tests

```
cargo test -p ol-sim --lib -- get_or_craft::tests::
```

- `npc_enqueue_seek_skips_blocked_tile`
- `npc_enqueue_craft_item_multi_step_use`
- `npc_enqueue_seek_or_craft_expands_multi_step_on_miss`
- `npc_enqueue_multi_step_skips_blocked_actor`
- `npc_enqueue_busy_moving_waits`
- `npc_enqueue_sticky_runtime_cooldown`
- `resolve_ex_scan_skips_blocked_product`
- `get_pile_obj_id_self_self_with_undo`
- `npc_enqueue_ex_pile_id_uses_pile_form`
- `npc_enqueue_ex_full_pile_tiles_skipped_in_multi_step`
- `npc_enqueue_blocked_by_ai_merge_skips_peer_claim`
- `get_or_craft_objs_from_scan_prefers_tile_num_slots`
- `full_pile_tiles_from_scan_collects_full_uses`

### Residual (next)

1. Full `SimState.blocked_by_ai` share into NPC thread (peer `craft_progress` proxy today; human/sticky rebuild stays on sim tick)
2. Full GetCraftAndDrop adze/bucket / dynamic WaterSourceIds → **AI-CRAFT-MULTI**
3. Depth-1 multi-step only (no nested craftItem expand loops)

### Haxe anchors

- `AiBase.GetOrCraftItem` ~6150–6219 → craftItem on miss
- `ObjectData.getPileObjId` ~1531–1538
- `AiBase.craftItem` / `craftItemHelper` ~6611–7130
- `AiBase.isObjectNotReachable` / hostile / blockedByAI ~9245–9281
- `ignoreFullPiles` + `numberOfUses >= numUses` pile search

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- get_or_craft::tests::
cargo check -p ol-server
```

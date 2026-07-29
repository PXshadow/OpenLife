# AI-JOB-SMITH-RESID — chisel content scan + NPC peer fidelity

**Date:** 2026-07-29  
**matrix_id:** `AI-JOB-SMITH-RESID`  
**chunk:** `smith_chisel_resid`  
**status_after:** PARTIAL → **core closed** (PlayerSnapshot home/last + multi-prof npc peer + wound + chisel load cache)

## Haxe anchors

- `ServerSettings.PatchObjectData` ~612–616: `description.contains('Chisel')` → `objectIdArrays[455]`
- `AiBase.doSmithing` ~3869: `countCurrentObjects(objectIdArrays[455])` for steel chisel stock
- `AiBase.countProfession` / `hasOrBecomeProfession('SMITH')` ~1284 / ~4481: same-home, skip wounded, max one SMITH
- `GlobalPlayerInstance.home.tx/ty` + `lastProfession` for peer filters
- `AiBase.doTimeStuffHelper` ~609 early sticky smith; ~617–621 critical shortCrafts (all AIs)

## Rust changes (this residual close)

### `PlayerSnapshot` (`ol-sim/src/player.rs`)

- `home_x` / `home_y` (Haxe `home.tx/ty`)
- `is_last_smith` / `is_last_baker` / `is_last_potter` / `is_last_shepherd` / `is_last_farm` / `is_last_fire_food`
- Filled from `Player` sticky runtimes in `snapshot()`

### Pure (`smith_profession.rs`)

- `SteelChiselFamilyTable` load-time `objectIdArrays[455]` cache
- `peer_home_coords` / `peer_is_wounded_from_held`
- `NpcSmithPeerRow::from_snapshot_fields` (snap sticky OR npc override)

### Pure (`profession_scan.rs`)

- `NpcProfessionPeerRow` multi-prof sticky row
- `npc_peer_count_for_kind` — smith/baker/pottery/shepherd/farm/fire from lightweight rows

### Live (`npc_ai.rs`)

- Home from `PlayerSnapshot.home_*` (not position proxy)
- Wound from `is_wound_object(content, held_id)`
- Peer roster from **all player_views** + profession_state sticky OR
- Peer count for **primary ladder kind** (not smith-only)
- Chisel extras from one-time `SteelChiselFamilyTable::from_content` at scheduler boot

## Tests

- `player_snapshot_includes_home_and_profession_sticky`
- `npc_smith_peer_wounded_excluded_and_home_fidelity`
- `npc_peer_count_for_kind_multi_prof_and_wounded`
- `steel_chisel_family_table_cache_extras`
- Prior: `chisel_content_scan_extends_family_and_stock` / `npc_smith_peer_rows_max_one_population`

```
cargo test -p ol-sim --lib -- smith_profession:: player_snapshot_includes npc_peer_count_for_kind steel_chisel
```

## Remaining gaps

- Live USE/DROP I/O polish (parent AI-JOB-SMITH-LIVE residual)
- Snapshot lacks hiddenWound alias (wound = any content wound on held)
- Player/sim path still re-scans chisel family each profession tick (npc cached)
- Ladder still uses single primary-kind peer_count for all steps (same as player path)
